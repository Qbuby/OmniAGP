use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

use crate::bug_report::{BugCategory, BugReport, Severity};
use crate::runner::GodotRunner;
use super::{QaTest, TestResult, TestType, TestVerdict};

pub struct NavigationTest {
    pub project_id: Uuid,
}

impl NavigationTest {
    pub fn new(project_id: Uuid) -> Self {
        Self { project_id }
    }

    fn generate_navigation_script(scenes: &[PathBuf]) -> String {
        let scene_loads: Vec<String> = scenes
            .iter()
            .filter_map(|s| s.to_str())
            .map(|s| {
                let res_path = s.replace('\\', "/");
                format!("    \"{res_path}\"")
            })
            .collect();

        format!(
            r#"extends SceneTree

var scenes_to_test = [
{}
]
var current_index = 0
var results = {{}}

func _init():
    print("NAV_TEST_START")
    _test_next_scene()

func _test_next_scene():
    if current_index >= scenes_to_test.size():
        _report_results()
        quit(0)
        return

    var scene_path = scenes_to_test[current_index]
    print("NAV_TEST_LOADING: " + scene_path)

    var scene = load(scene_path)
    if scene == null:
        results[scene_path] = "FAIL_LOAD"
        print("NAV_TEST_FAIL_LOAD: " + scene_path)
    else:
        var instance = scene.instantiate()
        if instance == null:
            results[scene_path] = "FAIL_INSTANCE"
            print("NAV_TEST_FAIL_INSTANCE: " + scene_path)
        else:
            root.add_child(instance)
            results[scene_path] = "PASS"
            print("NAV_TEST_PASS: " + scene_path)
            instance.queue_free()

    current_index += 1
    call_deferred("_test_next_scene")

func _report_results():
    print("NAV_TEST_COMPLETE")
    for scene_path in results:
        print("NAV_RESULT: " + scene_path + " = " + results[scene_path])
"#,
            scene_loads.join(",\n")
        )
    }
}

#[async_trait]
impl QaTest for NavigationTest {
    fn name(&self) -> &str {
        "navigation_test"
    }

    fn test_type(&self) -> TestType {
        TestType::Navigation
    }

    async fn execute(&self, runner: &GodotRunner) -> Result<TestResult> {
        info!("running navigation test: verifying all scenes are reachable");
        let start = std::time::Instant::now();

        let validation = runner.validate_project_structure().await?;

        if validation.scenes.is_empty() {
            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Warning,
                duration: start.elapsed(),
                message: "No scenes found in project".to_string(),
                details: serde_json::json!({"scene_count": 0}),
                bugs: vec![BugReport::new(
                    self.project_id,
                    BugCategory::DesignFlaw,
                    Severity::High,
                    "No scenes in project".to_string(),
                    "Project contains no .tscn scene files".to_string(),
                    self.name().to_string(),
                )],
            });
        }

        let script_content = Self::generate_navigation_script(&validation.scenes);
        let script_path = runner.config.project_path.join("_qa_nav_test.gd");
        tokio::fs::write(&script_path, &script_content).await?;

        let result = runner.run_with_script(&script_path).await?;

        let _ = tokio::fs::remove_file(&script_path).await;

        let duration = start.elapsed();
        let mut bugs = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut scene_results = serde_json::Map::new();

        for line in result.stdout.lines() {
            if line.starts_with("NAV_TEST_PASS:") {
                passed += 1;
                let scene = line.trim_start_matches("NAV_TEST_PASS:").trim();
                scene_results.insert(scene.to_string(), serde_json::json!("pass"));
            } else if line.starts_with("NAV_TEST_FAIL_LOAD:") {
                failed += 1;
                let scene = line.trim_start_matches("NAV_TEST_FAIL_LOAD:").trim();
                scene_results.insert(scene.to_string(), serde_json::json!("fail_load"));
                bugs.push(BugReport::new(
                    self.project_id,
                    BugCategory::AssetMissing,
                    Severity::High,
                    format!("Scene failed to load: {scene}"),
                    format!("Scene file exists but could not be loaded by Godot"),
                    self.name().to_string(),
                ));
            } else if line.starts_with("NAV_TEST_FAIL_INSTANCE:") {
                failed += 1;
                let scene = line.trim_start_matches("NAV_TEST_FAIL_INSTANCE:").trim();
                scene_results.insert(scene.to_string(), serde_json::json!("fail_instance"));
                bugs.push(BugReport::new(
                    self.project_id,
                    BugCategory::CodeBug,
                    Severity::High,
                    format!("Scene failed to instantiate: {scene}"),
                    format!("Scene loaded but instantiation failed — likely missing dependencies"),
                    self.name().to_string(),
                ));
            }
        }

        let verdict = if failed > 0 {
            TestVerdict::Fail
        } else if result.crashed {
            bugs.push(
                BugReport::new(
                    self.project_id,
                    BugCategory::CodeBug,
                    Severity::Critical,
                    "Navigation test crashed".to_string(),
                    "Godot crashed while testing scene navigation".to_string(),
                    self.name().to_string(),
                )
                .with_log(result.stderr.clone()),
            );
            TestVerdict::Fail
        } else {
            TestVerdict::Pass
        };

        Ok(TestResult {
            test_name: self.name().to_string(),
            test_type: self.test_type(),
            verdict,
            duration,
            message: format!("{passed} scenes passed, {failed} failed out of {} total", validation.scenes.len()),
            details: serde_json::json!({
                "total_scenes": validation.scenes.len(),
                "passed": passed,
                "failed": failed,
                "results": scene_results,
            }),
            bugs,
        })
    }
}
