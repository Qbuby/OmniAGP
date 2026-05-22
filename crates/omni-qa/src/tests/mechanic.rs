use anyhow::Result;
use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use crate::bug_report::{BugCategory, BugReport, Severity};
use crate::runner::GodotRunner;
use super::{QaTest, TestResult, TestType, TestVerdict};

pub struct MechanicTest {
    pub project_id: Uuid,
    pub mechanics: Vec<GameMechanic>,
}

#[derive(Debug, Clone)]
pub struct GameMechanic {
    pub name: String,
    pub input_action: String,
    pub expected_signal: String,
}

impl MechanicTest {
    pub fn new(project_id: Uuid, mechanics: Vec<GameMechanic>) -> Self {
        Self {
            project_id,
            mechanics,
        }
    }

    pub fn from_gdd(project_id: Uuid, gdd: &serde_json::Value) -> Self {
        let mut mechanics = Vec::new();

        if let Some(mechs) = gdd.get("mechanics").and_then(|m| m.as_array()) {
            for mech in mechs {
                let name = mech.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                let action = mech.get("input_action").and_then(|a| a.as_str()).unwrap_or("ui_accept");
                let signal = mech.get("expected_signal").and_then(|s| s.as_str()).unwrap_or("");

                mechanics.push(GameMechanic {
                    name: name.to_string(),
                    input_action: action.to_string(),
                    expected_signal: signal.to_string(),
                });
            }
        }

        if mechanics.is_empty() {
            mechanics = Self::default_mechanics();
        }

        Self::new(project_id, mechanics)
    }

    fn default_mechanics() -> Vec<GameMechanic> {
        vec![
            GameMechanic {
                name: "jump".to_string(),
                input_action: "ui_up".to_string(),
                expected_signal: "MECH_JUMP_OK".to_string(),
            },
            GameMechanic {
                name: "move_right".to_string(),
                input_action: "ui_right".to_string(),
                expected_signal: "MECH_MOVE_OK".to_string(),
            },
            GameMechanic {
                name: "interact".to_string(),
                input_action: "ui_accept".to_string(),
                expected_signal: "MECH_INTERACT_OK".to_string(),
            },
        ]
    }

    fn generate_mechanic_script(&self) -> String {
        let mut action_tests = String::new();
        for mech in &self.mechanics {
            action_tests.push_str(&format!(
                r#"
func _test_{name}():
    print("MECH_TEST_START: {name}")
    var action = InputEventAction.new()
    action.action = "{action}"
    action.pressed = true
    Input.parse_input_event(action)
    await get_tree().create_timer(0.5).timeout
    action.pressed = false
    Input.parse_input_event(action)
    await get_tree().create_timer(0.2).timeout
    print("MECH_TEST_DONE: {name}")
"#,
                name = mech.name,
                action = mech.input_action,
            ));
        }

        format!(
            r#"extends SceneTree

var test_index = 0
var test_methods = [{method_list}]

func _init():
    print("MECH_TEST_SUITE_START")
    call_deferred("_run_tests")

func _run_tests():
    await get_tree().create_timer(1.0).timeout
    for method in test_methods:
        await call(method)
    print("MECH_TEST_SUITE_COMPLETE")
    quit(0)
{action_tests}
"#,
            method_list = self
                .mechanics
                .iter()
                .map(|m| format!("\"_test_{}\"", m.name))
                .collect::<Vec<_>>()
                .join(", "),
            action_tests = action_tests,
        )
    }
}

#[async_trait]
impl QaTest for MechanicTest {
    fn name(&self) -> &str {
        "mechanic_test"
    }

    fn test_type(&self) -> TestType {
        TestType::Mechanic
    }

    async fn execute(&self, runner: &GodotRunner) -> Result<TestResult> {
        info!("running mechanic test: verifying {} mechanics", self.mechanics.len());
        let start = std::time::Instant::now();

        if self.mechanics.is_empty() {
            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Skipped,
                duration: start.elapsed(),
                message: "No mechanics defined to test".to_string(),
                details: serde_json::json!({}),
                bugs: Vec::new(),
            });
        }

        let script_content = self.generate_mechanic_script();
        let script_path = runner.config.project_path.join("_qa_mechanic_test.gd");
        tokio::fs::write(&script_path, &script_content).await?;

        let result = runner.run_with_script(&script_path).await?;

        let _ = tokio::fs::remove_file(&script_path).await;

        let duration = start.elapsed();
        let mut bugs = Vec::new();
        let mut tested = 0;
        let mut completed = 0;

        for line in result.stdout.lines() {
            if line.starts_with("MECH_TEST_START:") {
                tested += 1;
            } else if line.starts_with("MECH_TEST_DONE:") {
                completed += 1;
            }
        }

        if result.crashed {
            bugs.push(
                BugReport::new(
                    self.project_id,
                    BugCategory::CodeBug,
                    Severity::Critical,
                    "Game crashed during mechanic testing".to_string(),
                    format!("Crash occurred after testing {tested} mechanics ({completed} completed)"),
                    self.name().to_string(),
                )
                .with_log(result.stderr.clone()),
            );
        }

        let untested = self.mechanics.len() - completed;
        if untested > 0 && !result.crashed {
            for mech in self.mechanics.iter().skip(completed) {
                bugs.push(BugReport::new(
                    self.project_id,
                    BugCategory::CodeBug,
                    Severity::Medium,
                    format!("Mechanic '{}' could not be verified", mech.name),
                    format!(
                        "Input action '{}' did not produce expected behavior",
                        mech.input_action
                    ),
                    self.name().to_string(),
                ));
            }
        }

        let verdict = if result.crashed {
            TestVerdict::Fail
        } else if completed < self.mechanics.len() {
            TestVerdict::Warning
        } else {
            TestVerdict::Pass
        };

        Ok(TestResult {
            test_name: self.name().to_string(),
            test_type: self.test_type(),
            verdict,
            duration,
            message: format!("{completed}/{} mechanics completed", self.mechanics.len()),
            details: serde_json::json!({
                "total_mechanics": self.mechanics.len(),
                "tested": tested,
                "completed": completed,
                "crashed": result.crashed,
            }),
            bugs,
        })
    }
}
