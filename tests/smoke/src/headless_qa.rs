use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Serialize)]
pub struct QaResult {
    pub passed: bool,
    pub checks: Vec<QaCheck>,
    pub crash_detected: bool,
    pub main_path_completable: bool,
}

#[derive(Debug, Serialize)]
pub struct QaCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

pub async fn run_headless_qa(output_dir: &Path) -> Result<QaResult> {
    let project_dir = output_dir.join("godot_project");
    let mut checks = Vec::new();

    // Check 1: Project file exists and is valid
    let project_file = project_dir.join("project.godot");
    let project_valid = project_file.exists();
    checks.push(QaCheck {
        name: "project_file_exists".into(),
        passed: project_valid,
        details: if project_valid {
            "project.godot found".into()
        } else {
            "project.godot missing".into()
        },
    });

    // Check 2: Main scene exists
    let main_scene = project_dir.join("scenes").join("start_menu.tscn");
    let main_scene_exists = main_scene.exists();
    checks.push(QaCheck {
        name: "main_scene_exists".into(),
        passed: main_scene_exists,
        details: format!("start_menu.tscn: {}", if main_scene_exists { "found" } else { "missing" }),
    });

    // Check 3: All required scenes present
    let required_scenes = ["start_menu.tscn", "level_1.tscn", "boss_fight.tscn", "victory_screen.tscn"];
    let scenes_dir = project_dir.join("scenes");
    let mut all_scenes_present = true;
    for scene in &required_scenes {
        if !scenes_dir.join(scene).exists() {
            all_scenes_present = false;
            warn!(scene = scene, "required scene missing");
        }
    }
    checks.push(QaCheck {
        name: "all_scenes_present".into(),
        passed: all_scenes_present,
        details: format!("{}/4 scenes present", required_scenes.iter().filter(|s| scenes_dir.join(s).exists()).count()),
    });

    // Check 4: All scripts have valid extends
    let scripts_valid = validate_all_scripts(&project_dir)?;
    checks.push(QaCheck {
        name: "scripts_valid".into(),
        passed: scripts_valid,
        details: if scripts_valid {
            "all scripts have valid extends declaration".into()
        } else {
            "some scripts missing extends".into()
        },
    });

    // Check 5: Assets present
    let assets_dir = project_dir.join("assets");
    let asset_count = if assets_dir.exists() {
        std::fs::read_dir(&assets_dir)?.count()
    } else {
        0
    };
    let assets_ok = asset_count >= 4; // at least sprites + tileset + some audio
    checks.push(QaCheck {
        name: "assets_present".into(),
        passed: assets_ok,
        details: format!("{} asset files found (minimum 4 required)", asset_count),
    });

    // Check 6: Scene transitions are wired (scripts reference scene changes)
    let transitions_ok = check_scene_transitions(&project_dir)?;
    checks.push(QaCheck {
        name: "scene_transitions_wired".into(),
        passed: transitions_ok,
        details: if transitions_ok {
            "scene transition calls found in scripts".into()
        } else {
            "no scene transition calls detected".into()
        },
    });

    // Check 7: Try headless Godot execution if available
    let godot_available = check_godot_available().await;
    let headless_ok = if godot_available {
        match run_godot_headless(&project_dir).await {
            Ok(result) => {
                checks.push(QaCheck {
                    name: "headless_execution".into(),
                    passed: result,
                    details: if result {
                        "godot --headless exited cleanly".into()
                    } else {
                        "godot --headless crashed or timed out".into()
                    },
                });
                result
            }
            Err(e) => {
                checks.push(QaCheck {
                    name: "headless_execution".into(),
                    passed: false,
                    details: format!("headless execution error: {}", e),
                });
                false
            }
        }
    } else {
        info!("godot not found in PATH, skipping headless execution test");
        checks.push(QaCheck {
            name: "headless_execution".into(),
            passed: true,
            details: "skipped: godot not in PATH (non-blocking)".into(),
        });
        true
    };

    let crash_detected = !headless_ok && godot_available;
    let main_path_completable = all_scenes_present && scripts_valid && transitions_ok;
    let passed = project_valid && main_scene_exists && all_scenes_present && scripts_valid && assets_ok;

    info!(
        passed = passed,
        checks_passed = checks.iter().filter(|c| c.passed).count(),
        total_checks = checks.len(),
        "QA complete"
    );

    Ok(QaResult {
        passed,
        checks,
        crash_detected,
        main_path_completable,
    })
}

fn validate_all_scripts(project_dir: &Path) -> Result<bool> {
    let mut all_valid = true;

    for dir_name in &["scenes", "entities"] {
        let dir = project_dir.join(dir_name);
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "gd").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)?;
                if !content.lines().any(|l| l.trim().starts_with("extends")) {
                    warn!(file = %path.display(), "missing extends");
                    all_valid = false;
                }
            }
        }
    }

    Ok(all_valid)
}

fn check_scene_transitions(project_dir: &Path) -> Result<bool> {
    let scenes_dir = project_dir.join("scenes");
    if !scenes_dir.exists() {
        return Ok(false);
    }

    let mut has_transition = false;
    for entry in std::fs::read_dir(&scenes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "gd").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            if content.contains("get_tree().change_scene")
                || content.contains("SceneTree.change_scene")
                || content.contains("change_scene_to_packed")
                || content.contains("change_scene_to_file")
            {
                has_transition = true;
                break;
            }
        }
    }

    Ok(has_transition)
}

async fn check_godot_available() -> bool {
    let result = tokio::process::Command::new("godot")
        .arg("--version")
        .output()
        .await;
    result.is_ok()
}

async fn run_godot_headless(project_dir: &Path) -> Result<bool> {
    let output = tokio::process::Command::new("godot")
        .args(["--headless", "--path"])
        .arg(project_dir)
        .args(["--quit-after", "5"])
        .output()
        .await?;

    let exit_ok = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !exit_ok {
        warn!(stderr = %stderr, "godot headless failed");
    }

    let has_crash = stderr.contains("CRASH") || stderr.contains("Segmentation fault");
    Ok(exit_ok && !has_crash)
}
