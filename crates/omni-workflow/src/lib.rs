use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GddStatus {
    Draft,
    InReview,
    Approved,
    Generating,
    Qa,
    Published,
}

impl GddStatus {
    pub fn allowed_transitions(&self) -> &'static [GddStatus] {
        match self {
            GddStatus::Draft => &[GddStatus::InReview],
            GddStatus::InReview => &[GddStatus::Draft, GddStatus::Approved],
            GddStatus::Approved => &[GddStatus::Draft, GddStatus::Generating],
            GddStatus::Generating => &[GddStatus::Qa, GddStatus::Draft],
            GddStatus::Qa => &[GddStatus::Published, GddStatus::Draft],
            GddStatus::Published => &[GddStatus::Draft],
        }
    }

    pub fn can_transition_to(&self, target: GddStatus) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("invalid transition from {from:?} to {to:?}")]
    InvalidTransition { from: GddStatus, to: GddStatus },
    #[error("insufficient approvals: {current}/{required}")]
    InsufficientApprovals { current: usize, required: usize },
    #[error("user {0} is not a reviewer")]
    NotAReviewer(Uuid),
    #[error("review already submitted by {0}")]
    DuplicateReview(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalConfig {
    pub required_approvals: usize,
    pub reviewers: Vec<Uuid>,
}

impl Default for ApprovalConfig {
    fn default() -> Self {
        Self {
            required_approvals: 1,
            reviewers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: Uuid,
    pub author_id: Uuid,
    pub field_path: String,
    pub content: String,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewDecision {
    Approve,
    RequestChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSubmission {
    pub reviewer_id: Uuid,
    pub decision: ReviewDecision,
    pub comment: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub id: Uuid,
    pub gdd_id: Uuid,
    pub version_id: Uuid,
    pub requested_by: Uuid,
    pub reviewers: Vec<Uuid>,
    pub submissions: Vec<ReviewSubmission>,
    pub comments: Vec<ReviewComment>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub gdd_id: Uuid,
    pub status: GddStatus,
    pub approval_config: ApprovalConfig,
    pub current_review: Option<ReviewRequest>,
    pub history: Vec<StatusTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransition {
    pub from: GddStatus,
    pub to: GddStatus,
    pub triggered_by: Uuid,
    pub timestamp: DateTime<Utc>,
}

impl WorkflowState {
    pub fn new(gdd_id: Uuid, approval_config: ApprovalConfig) -> Self {
        Self {
            gdd_id,
            status: GddStatus::Draft,
            approval_config,
            current_review: None,
            history: Vec::new(),
        }
    }

    pub fn submit_for_review(
        &mut self,
        requested_by: Uuid,
        version_id: Uuid,
    ) -> Result<(), WorkflowError> {
        if !self.status.can_transition_to(GddStatus::InReview) {
            return Err(WorkflowError::InvalidTransition {
                from: self.status,
                to: GddStatus::InReview,
            });
        }

        let review = ReviewRequest {
            id: Uuid::new_v4(),
            gdd_id: self.gdd_id,
            version_id,
            requested_by,
            reviewers: self.approval_config.reviewers.clone(),
            submissions: Vec::new(),
            comments: Vec::new(),
            created_at: Utc::now(),
        };

        self.current_review = Some(review);
        self.transition(GddStatus::InReview, requested_by);
        Ok(())
    }

    pub fn submit_review(
        &mut self,
        reviewer_id: Uuid,
        decision: ReviewDecision,
        comment: Option<String>,
    ) -> Result<(), WorkflowError> {
        let review = self
            .current_review
            .as_mut()
            .ok_or(WorkflowError::InvalidTransition {
                from: self.status,
                to: GddStatus::Approved,
            })?;

        if !review.reviewers.is_empty() && !review.reviewers.contains(&reviewer_id) {
            return Err(WorkflowError::NotAReviewer(reviewer_id));
        }

        if review.submissions.iter().any(|s| s.reviewer_id == reviewer_id) {
            return Err(WorkflowError::DuplicateReview(reviewer_id));
        }

        review.submissions.push(ReviewSubmission {
            reviewer_id,
            decision,
            comment,
            submitted_at: Utc::now(),
        });

        Ok(())
    }

    pub fn try_approve(&mut self, triggered_by: Uuid) -> Result<(), WorkflowError> {
        let approvals = self
            .current_review
            .as_ref()
            .map(|r| {
                r.submissions
                    .iter()
                    .filter(|s| s.decision == ReviewDecision::Approve)
                    .count()
            })
            .unwrap_or(0);

        let required = self.approval_config.required_approvals;
        if approvals < required {
            return Err(WorkflowError::InsufficientApprovals {
                current: approvals,
                required,
            });
        }

        self.transition(GddStatus::Approved, triggered_by);
        Ok(())
    }

    pub fn start_generation(&mut self, triggered_by: Uuid) -> Result<(), WorkflowError> {
        if !self.status.can_transition_to(GddStatus::Generating) {
            return Err(WorkflowError::InvalidTransition {
                from: self.status,
                to: GddStatus::Generating,
            });
        }
        self.transition(GddStatus::Generating, triggered_by);
        Ok(())
    }

    pub fn move_to_qa(&mut self, triggered_by: Uuid) -> Result<(), WorkflowError> {
        if !self.status.can_transition_to(GddStatus::Qa) {
            return Err(WorkflowError::InvalidTransition {
                from: self.status,
                to: GddStatus::Qa,
            });
        }
        self.transition(GddStatus::Qa, triggered_by);
        Ok(())
    }

    pub fn publish(&mut self, triggered_by: Uuid) -> Result<(), WorkflowError> {
        if !self.status.can_transition_to(GddStatus::Published) {
            return Err(WorkflowError::InvalidTransition {
                from: self.status,
                to: GddStatus::Published,
            });
        }
        self.transition(GddStatus::Published, triggered_by);
        Ok(())
    }

    pub fn reject_to_draft(&mut self, triggered_by: Uuid) -> Result<(), WorkflowError> {
        if !self.status.can_transition_to(GddStatus::Draft) {
            return Err(WorkflowError::InvalidTransition {
                from: self.status,
                to: GddStatus::Draft,
            });
        }
        self.current_review = None;
        self.transition(GddStatus::Draft, triggered_by);
        Ok(())
    }

    pub fn add_review_comment(
        &mut self,
        author_id: Uuid,
        field_path: String,
        content: String,
    ) -> Option<Uuid> {
        let review = self.current_review.as_mut()?;
        let comment = ReviewComment {
            id: Uuid::new_v4(),
            author_id,
            field_path,
            content,
            resolved: false,
            created_at: Utc::now(),
        };
        let id = comment.id;
        review.comments.push(comment);
        Some(id)
    }

    fn transition(&mut self, to: GddStatus, triggered_by: Uuid) {
        self.history.push(StatusTransition {
            from: self.status,
            to,
            triggered_by,
            timestamp: Utc::now(),
        });
        self.status = to;
    }
}

pub struct WorkflowEngine {
    workflows: HashMap<Uuid, WorkflowState>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
        }
    }

    pub fn create_workflow(&mut self, gdd_id: Uuid, config: ApprovalConfig) -> &WorkflowState {
        let state = WorkflowState::new(gdd_id, config);
        self.workflows.insert(gdd_id, state);
        self.workflows.get(&gdd_id).unwrap()
    }

    pub fn get(&self, gdd_id: &Uuid) -> Option<&WorkflowState> {
        self.workflows.get(gdd_id)
    }

    pub fn get_mut(&mut self, gdd_id: &Uuid) -> Option<&mut WorkflowState> {
        self.workflows.get_mut(gdd_id)
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}
