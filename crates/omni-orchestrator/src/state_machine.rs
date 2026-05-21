use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectState {
    Draft,
    Clarifying,
    Planning,
    Executing,
    Testing,
    Delivered,
    Failed,
}

impl ProjectState {
    pub fn valid_transitions(&self) -> &[ProjectState] {
        match self {
            Self::Draft => &[Self::Clarifying, Self::Planning],
            Self::Clarifying => &[Self::Planning, Self::Draft],
            Self::Planning => &[Self::Executing, Self::Clarifying],
            Self::Executing => &[Self::Testing, Self::Failed, Self::Planning],
            Self::Testing => &[Self::Delivered, Self::Executing, Self::Failed],
            Self::Delivered => &[],
            Self::Failed => &[Self::Draft, Self::Planning],
        }
    }

    pub fn can_transition_to(&self, target: ProjectState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

impl std::fmt::Display for ProjectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "DRAFT"),
            Self::Clarifying => write!(f, "CLARIFYING"),
            Self::Planning => write!(f, "PLANNING"),
            Self::Executing => write!(f, "EXECUTING"),
            Self::Testing => write!(f, "TESTING"),
            Self::Delivered => write!(f, "DELIVERED"),
            Self::Failed => write!(f, "FAILED"),
        }
    }
}

#[derive(Clone)]
pub struct ProjectStateMachine {
    state: Arc<RwLock<ProjectState>>,
}

impl ProjectStateMachine {
    pub fn new(initial: ProjectState) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
        }
    }

    pub async fn current(&self) -> ProjectState {
        *self.state.read().await
    }

    pub async fn transition(&self, target: ProjectState) -> Result<ProjectState> {
        let mut state = self.state.write().await;
        if !state.can_transition_to(target) {
            bail!(
                "invalid state transition: {} -> {}",
                *state,
                target
            );
        }
        *state = target;
        Ok(target)
    }

    pub async fn force_state(&self, target: ProjectState) {
        let mut state = self.state.write().await;
        *state = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_transitions() {
        let sm = ProjectStateMachine::new(ProjectState::Draft);
        assert_eq!(sm.current().await, ProjectState::Draft);

        sm.transition(ProjectState::Planning).await.unwrap();
        assert_eq!(sm.current().await, ProjectState::Planning);

        sm.transition(ProjectState::Executing).await.unwrap();
        assert_eq!(sm.current().await, ProjectState::Executing);

        sm.transition(ProjectState::Testing).await.unwrap();
        sm.transition(ProjectState::Delivered).await.unwrap();
        assert_eq!(sm.current().await, ProjectState::Delivered);
    }

    #[tokio::test]
    async fn test_invalid_transition() {
        let sm = ProjectStateMachine::new(ProjectState::Draft);
        let result = sm.transition(ProjectState::Delivered).await;
        assert!(result.is_err());
    }
}
