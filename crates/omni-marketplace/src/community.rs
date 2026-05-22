use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub plugin_count: u32,
    pub template_count: u32,
    pub asset_pack_count: u32,
    pub total_downloads: u64,
    pub reputation_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub id: Uuid,
    pub contributor_id: Uuid,
    pub contribution_type: ContributionType,
    pub name: String,
    pub version: String,
    pub status: ContributionStatus,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContributionType {
    Plugin,
    Template,
    AssetPack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContributionStatus {
    Submitted,
    SecurityScan,
    CompatibilityTest,
    InReview,
    Approved,
    Rejected(String),
    Published,
}

impl ContributionStatus {
    pub fn can_transition_to(&self, next: &Self) -> bool {
        matches!(
            (self, next),
            (Self::Submitted, Self::SecurityScan)
                | (Self::SecurityScan, Self::CompatibilityTest)
                | (Self::SecurityScan, Self::Rejected(_))
                | (Self::CompatibilityTest, Self::InReview)
                | (Self::CompatibilityTest, Self::Rejected(_))
                | (Self::InReview, Self::Approved)
                | (Self::InReview, Self::Rejected(_))
                | (Self::Approved, Self::Published)
        )
    }
}
