use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub id: Uuid,
    pub bug_type: BugType,
    pub severity: Severity,
    pub description: String,
    pub stack_trace: Option<String>,
    pub related_files: Vec<String>,
    pub error_message: Option<String>,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BugType {
    CodeBug,
    AssetMissing,
    DesignFlaw,
    PerformanceIssue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixAttempt {
    pub round: u32,
    pub strategy: String,
    pub degradation_level: DegradationLevel,
    pub patch: Patch,
    pub result: FixOutcome,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub file_path: String,
    pub original: String,
    pub modified: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixOutcome {
    Success,
    PartialFix,
    Failed,
    Regression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationLevel {
    Retry,
    AugmentedContext,
    TemplateFallback,
    Escalate,
}

impl DegradationLevel {
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Retry => Some(Self::AugmentedContext),
            Self::AugmentedContext => Some(Self::TemplateFallback),
            Self::TemplateFallback => Some(Self::Escalate),
            Self::Escalate => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub bug_id: Uuid,
    pub success: bool,
    pub attempts: Vec<FixAttempt>,
    pub final_status: FinalStatus,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalStatus {
    Fixed,
    PartiallyFixed,
    Escalated { reason: String },
    MaxRetriesExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixLog {
    pub entries: Vec<FixResult>,
}

impl FixLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, result: FixResult) {
        self.entries.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let successes = self.entries.iter().filter(|e| e.success).count();
        successes as f64 / self.entries.len() as f64
    }

    pub fn avg_duration_ms(&self) -> u64 {
        if self.entries.is_empty() {
            return 0;
        }
        let total: u64 = self.entries.iter().map(|e| e.total_duration_ms).sum();
        total / self.entries.len() as u64
    }
}

impl Default for FixLog {
    fn default() -> Self {
        Self::new()
    }
}
