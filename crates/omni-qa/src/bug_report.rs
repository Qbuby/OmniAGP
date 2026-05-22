use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BugCategory {
    CodeBug,
    AssetMissing,
    DesignFlaw,
    PerformanceIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub id: Uuid,
    pub project_id: Uuid,
    pub category: BugCategory,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub reproduction_steps: Vec<String>,
    pub test_name: String,
    pub log_snippet: Option<String>,
    pub screenshot_path: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl BugReport {
    pub fn new(
        project_id: Uuid,
        category: BugCategory,
        severity: Severity,
        title: String,
        description: String,
        test_name: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            category,
            severity,
            title,
            description,
            reproduction_steps: Vec::new(),
            test_name,
            log_snippet: None,
            screenshot_path: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_log(mut self, log: String) -> Self {
        self.log_snippet = Some(log);
        self
    }

    pub fn with_steps(mut self, steps: Vec<String>) -> Self {
        self.reproduction_steps = steps;
        self
    }
}
