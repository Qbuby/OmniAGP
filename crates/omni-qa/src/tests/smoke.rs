use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

use crate::bug_report::{BugCategory, BugReport, Severity};
use crate::runner::GodotRunner;
use super::{QaTest, TestResult, TestType, TestVerdict};

pub struct SmokeTest {
    pub project_id: Uuid,
    pub run_duration: Duration,
}

impl SmokeTest {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project_id,
            run_duration: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl QaTest for SmokeTest {
    fn name(&self) -> &str {
        "smoke_test"
    }

    fn test_type(&self) -> TestType {
        TestType::Smoke
    }

    async fn execute(&self, runner: &GodotRunner) -> Result<TestResult> {
        info!("running smoke test: launch game for {:?}", self.run_duration);
        let start = std::time::Instant::now();

        let result = runner.run_project().await?;
        let duration = start.elapsed();

        let mut bugs = Vec::new();

        if result.crashed {
            bugs.push(
                BugReport::new(
                    self.project_id,
                    BugCategory::CodeBug,
                    Severity::Critical,
                    "Game crashes on startup".to_string(),
                    format!(
                        "Game crashed within {:?} of launch. Exit code: {:?}",
                        duration, result.exit_code
                    ),
                    self.name().to_string(),
                )
                .with_log(result.stderr.clone()),
            );

            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Fail,
                duration,
                message: "Game crashed on startup".to_string(),
                details: serde_json::json!({
                    "exit_code": result.exit_code,
                    "errors": result.extract_errors(),
                }),
                bugs,
            });
        }

        if result.timed_out {
            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Pass,
                duration,
                message: format!("Game ran for {:?} without crashing (timed out as expected)", self.run_duration),
                details: serde_json::json!({"ran_full_duration": true}),
                bugs,
            });
        }

        if result.has_errors() {
            let errors = result.extract_errors();
            bugs.push(
                BugReport::new(
                    self.project_id,
                    BugCategory::CodeBug,
                    Severity::High,
                    "Script errors during startup".to_string(),
                    format!("Found {} errors in game output", errors.len()),
                    self.name().to_string(),
                )
                .with_log(errors.join("\n")),
            );

            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Warning,
                duration,
                message: "Game launched but produced errors".to_string(),
                details: serde_json::json!({"errors": errors}),
                bugs,
            });
        }

        Ok(TestResult {
            test_name: self.name().to_string(),
            test_type: self.test_type(),
            verdict: TestVerdict::Pass,
            duration,
            message: "Game launched and ran without errors".to_string(),
            details: serde_json::json!({"clean_run": true}),
            bugs,
        })
    }
}
