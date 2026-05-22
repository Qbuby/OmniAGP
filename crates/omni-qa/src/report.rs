use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bug_report::BugReport;
use crate::tests::{TestResult, TestVerdict};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QaVerdict {
    Pass,
    Fail,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaReport {
    pub id: Uuid,
    pub project_id: Uuid,
    pub verdict: QaVerdict,
    pub timestamp: DateTime<Utc>,
    pub total_duration_ms: u64,
    pub summary: ReportSummary,
    pub test_results: Vec<TestResult>,
    pub bugs: Vec<BugReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub skipped: usize,
    pub total_bugs: usize,
    pub critical_bugs: usize,
}

impl QaReport {
    pub fn from_results(project_id: Uuid, results: Vec<TestResult>, total_duration_ms: u64) -> Self {
        let mut all_bugs: Vec<BugReport> = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut warnings = 0;
        let mut skipped = 0;

        for result in &results {
            match result.verdict {
                TestVerdict::Pass => passed += 1,
                TestVerdict::Fail => failed += 1,
                TestVerdict::Warning => warnings += 1,
                TestVerdict::Skipped => skipped += 1,
            }
            all_bugs.extend(result.bugs.clone());
        }

        let critical_bugs = all_bugs
            .iter()
            .filter(|b| b.severity == crate::bug_report::Severity::Critical)
            .count();

        let verdict = if failed > 0 || critical_bugs > 0 {
            QaVerdict::Fail
        } else if warnings > 0 {
            QaVerdict::Warning
        } else {
            QaVerdict::Pass
        };

        Self {
            id: Uuid::new_v4(),
            project_id,
            verdict,
            timestamp: Utc::now(),
            total_duration_ms,
            summary: ReportSummary {
                total_tests: results.len(),
                passed,
                failed,
                warnings,
                skipped,
                total_bugs: all_bugs.len(),
                critical_bugs,
            },
            test_results: results,
            bugs: all_bugs,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}
