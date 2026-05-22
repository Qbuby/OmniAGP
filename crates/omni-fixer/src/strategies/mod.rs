use anyhow::Result;
use async_trait::async_trait;

use crate::types::{BugReport, DegradationLevel, Patch};

pub mod code_bug;
pub mod asset_missing;
pub mod design_flaw;
pub mod performance;

#[async_trait]
pub trait FixStrategy: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_fix(
        &self,
        bug: &BugReport,
        level: DegradationLevel,
        previous_attempts: &[Patch],
    ) -> Result<Patch>;
}
