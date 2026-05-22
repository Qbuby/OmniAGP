pub mod smoke;
pub mod navigation;
pub mod mechanic;
pub mod performance;
pub mod asset_integrity;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::bug_report::BugReport;
use crate::runner::GodotRunner;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestVerdict {
    Pass,
    Fail,
    Warning,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub test_type: TestType,
    pub verdict: TestVerdict,
    pub duration: Duration,
    pub message: String,
    pub details: serde_json::Value,
    pub bugs: Vec<BugReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    Smoke,
    Navigation,
    Mechanic,
    Performance,
    AssetIntegrity,
}

#[async_trait]
pub trait QaTest: Send + Sync {
    fn name(&self) -> &str;
    fn test_type(&self) -> TestType;
    async fn execute(&self, runner: &GodotRunner) -> Result<TestResult>;
}
