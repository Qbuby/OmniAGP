use anyhow::Result;
use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use crate::bug_report::{BugCategory, BugReport, Severity};
use crate::runner::GodotRunner;
use super::{QaTest, TestResult, TestType, TestVerdict};

pub struct AssetIntegrityTest {
    pub project_id: Uuid,
}

impl AssetIntegrityTest {
    pub fn new(project_id: Uuid) -> Self {
        Self { project_id }
    }
}

#[async_trait]
impl QaTest for AssetIntegrityTest {
    fn name(&self) -> &str {
        "asset_integrity_test"
    }

    fn test_type(&self) -> TestType {
        TestType::AssetIntegrity
    }

    async fn execute(&self, runner: &GodotRunner) -> Result<TestResult> {
        info!("running asset integrity test: checking all referenced assets exist");
        let start = std::time::Instant::now();

        let validation = runner.validate_project_structure().await?;
        let duration = start.elapsed();

        let mut bugs = Vec::new();

        if !validation.has_project_file {
            bugs.push(BugReport::new(
                self.project_id,
                BugCategory::AssetMissing,
                Severity::Critical,
                "Missing project.godot file".to_string(),
                "The project root does not contain a project.godot file".to_string(),
                self.name().to_string(),
            ));
        }

        for missing_ref in &validation.missing_references {
            bugs.push(BugReport::new(
                self.project_id,
                BugCategory::AssetMissing,
                Severity::High,
                format!("Missing asset: {}", missing_ref.referenced_path),
                format!(
                    "Referenced in {} but file does not exist on disk",
                    missing_ref.source_file.display()
                ),
                self.name().to_string(),
            ));
        }

        if validation.scene_count == 0 {
            bugs.push(BugReport::new(
                self.project_id,
                BugCategory::AssetMissing,
                Severity::High,
                "No scene files found".to_string(),
                "Project contains no .tscn files".to_string(),
                self.name().to_string(),
            ));
        }

        if validation.script_count == 0 {
            bugs.push(BugReport::new(
                self.project_id,
                BugCategory::DesignFlaw,
                Severity::Medium,
                "No script files found".to_string(),
                "Project contains no .gd script files — game may have no logic".to_string(),
                self.name().to_string(),
            ));
        }

        let verdict = if bugs.iter().any(|b| b.severity == Severity::Critical) {
            TestVerdict::Fail
        } else if !bugs.is_empty() {
            TestVerdict::Warning
        } else {
            TestVerdict::Pass
        };

        Ok(TestResult {
            test_name: self.name().to_string(),
            test_type: self.test_type(),
            verdict,
            duration,
            message: format!(
                "{} scenes, {} scripts, {} resources, {} missing references",
                validation.scene_count,
                validation.script_count,
                validation.resource_count,
                validation.missing_references.len(),
            ),
            details: serde_json::json!({
                "has_project_file": validation.has_project_file,
                "scene_count": validation.scene_count,
                "script_count": validation.script_count,
                "resource_count": validation.resource_count,
                "missing_references": validation.missing_references.len(),
                "missing_details": validation.missing_references.iter().map(|r| {
                    serde_json::json!({
                        "source": r.source_file.display().to_string(),
                        "missing": &r.referenced_path,
                    })
                }).collect::<Vec<_>>(),
            }),
            bugs,
        })
    }
}
