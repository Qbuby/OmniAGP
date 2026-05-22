use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

use crate::report::QaReport;
use crate::runner::{GodotRunner, GodotRunnerConfig};
use crate::tests::asset_integrity::AssetIntegrityTest;
use crate::tests::mechanic::MechanicTest;
use crate::tests::navigation::NavigationTest;
use crate::tests::performance::PerformanceTest;
use crate::tests::smoke::SmokeTest;
use crate::tests::{QaTest, TestResult};

pub struct QaAgent {
    project_id: Uuid,
    runner: GodotRunner,
    gdd: Option<serde_json::Value>,
}

pub struct QaAgentConfig {
    pub project_id: Uuid,
    pub project_path: PathBuf,
    pub godot_binary: PathBuf,
    pub gdd: Option<serde_json::Value>,
}

impl QaAgent {
    pub fn new(config: QaAgentConfig) -> Self {
        let runner_config = GodotRunnerConfig {
            godot_binary: config.godot_binary,
            project_path: config.project_path,
            headless: true,
            ..Default::default()
        };

        Self {
            project_id: config.project_id,
            runner: GodotRunner::new(runner_config),
            gdd: config.gdd,
        }
    }

    pub async fn run_full_qa(&self) -> Result<QaReport> {
        info!(project_id = %self.project_id, "starting full QA suite");
        let start = std::time::Instant::now();

        let tests = self.build_test_plan();
        let mut results: Vec<TestResult> = Vec::new();

        for test in &tests {
            info!(test = test.name(), "executing test");
            match test.execute(&self.runner).await {
                Ok(result) => {
                    info!(
                        test = test.name(),
                        verdict = ?result.verdict,
                        bugs = result.bugs.len(),
                        "test complete"
                    );
                    results.push(result);
                }
                Err(e) => {
                    info!(test = test.name(), error = %e, "test execution failed");
                    results.push(TestResult {
                        test_name: test.name().to_string(),
                        test_type: test.test_type(),
                        verdict: crate::tests::TestVerdict::Fail,
                        duration: std::time::Duration::ZERO,
                        message: format!("Test execution error: {e}"),
                        details: serde_json::json!({"error": e.to_string()}),
                        bugs: vec![crate::bug_report::BugReport::new(
                            self.project_id,
                            crate::bug_report::BugCategory::CodeBug,
                            crate::bug_report::Severity::Critical,
                            format!("Test '{}' failed to execute", test.name()),
                            e.to_string(),
                            test.name().to_string(),
                        )],
                    });
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        let report = QaReport::from_results(self.project_id, results, total_duration_ms);

        info!(
            verdict = ?report.verdict,
            total_bugs = report.summary.total_bugs,
            duration_ms = total_duration_ms,
            "QA suite complete"
        );

        Ok(report)
    }

    pub async fn run_smoke_only(&self) -> Result<QaReport> {
        let start = std::time::Instant::now();
        let test = SmokeTest::new(self.project_id);
        let result = test.execute(&self.runner).await?;
        let total_duration_ms = start.elapsed().as_millis() as u64;
        Ok(QaReport::from_results(self.project_id, vec![result], total_duration_ms))
    }

    fn build_test_plan(&self) -> Vec<Box<dyn QaTest>> {
        let mut tests: Vec<Box<dyn QaTest>> = Vec::new();

        tests.push(Box::new(AssetIntegrityTest::new(self.project_id)));
        tests.push(Box::new(SmokeTest::new(self.project_id)));
        tests.push(Box::new(NavigationTest::new(self.project_id)));

        let mechanic_test = match &self.gdd {
            Some(gdd) => MechanicTest::from_gdd(self.project_id, gdd),
            None => MechanicTest::new(self.project_id, Vec::new()),
        };
        tests.push(Box::new(mechanic_test));

        tests.push(Box::new(PerformanceTest::new(self.project_id)));

        tests
    }

    pub fn runner(&self) -> &GodotRunner {
        &self.runner
    }
}
