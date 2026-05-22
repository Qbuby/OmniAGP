use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodotRunnerConfig {
    pub godot_binary: PathBuf,
    pub project_path: PathBuf,
    pub headless: bool,
    pub timeout: Duration,
    pub resolution: (u32, u32),
}

impl Default for GodotRunnerConfig {
    fn default() -> Self {
        Self {
            godot_binary: PathBuf::from("godot4"),
            project_path: PathBuf::new(),
            headless: true,
            timeout: Duration::from_secs(60),
            resolution: (1280, 720),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub crashed: bool,
    pub timed_out: bool,
}

impl RunResult {
    pub fn has_errors(&self) -> bool {
        self.crashed
            || self.timed_out
            || self.stderr.contains("ERROR")
            || self.stderr.contains("SCRIPT ERROR")
    }

    pub fn has_warnings(&self) -> bool {
        self.stderr.contains("WARNING") || self.stdout.contains("WARNING")
    }

    pub fn extract_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for line in self.stderr.lines().chain(self.stdout.lines()) {
            if line.contains("ERROR") || line.contains("SCRIPT ERROR") || line.contains("FATAL") {
                errors.push(line.to_string());
            }
        }
        errors
    }
}

pub struct GodotRunner {
    pub config: GodotRunnerConfig,
}

impl GodotRunner {
    pub fn new(config: GodotRunnerConfig) -> Self {
        Self { config }
    }

    pub async fn run_scene(&self, scene_path: &str) -> Result<RunResult> {
        self.run_with_args(&["--path", self.config.project_path.to_str().unwrap_or("."), scene_path]).await
    }

    pub async fn run_project(&self) -> Result<RunResult> {
        self.run_with_args(&["--path", self.config.project_path.to_str().unwrap_or(".")]).await
    }

    pub async fn run_with_script(&self, script_path: &Path) -> Result<RunResult> {
        self.run_with_args(&[
            "--path",
            self.config.project_path.to_str().unwrap_or("."),
            "--script",
            script_path.to_str().unwrap_or(""),
        ])
        .await
    }

    async fn run_with_args(&self, extra_args: &[&str]) -> Result<RunResult> {
        let mut args = Vec::new();

        if self.config.headless {
            args.push("--headless");
        }

        args.push("--rendering-driver");
        args.push("vulkan");

        let res = format!("{}x{}", self.config.resolution.0, self.config.resolution.1);
        args.push("--resolution");
        args.push(&res);

        args.extend_from_slice(extra_args);

        info!(binary = %self.config.godot_binary.display(), ?args, "launching godot");

        let start = std::time::Instant::now();

        let result = timeout(self.config.timeout, async {
            Command::new(&self.config.godot_binary)
                .args(&args)
                .output()
                .await
                .context("failed to execute godot binary")
        })
        .await;

        let duration = start.elapsed();

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let crashed = exit_code.map_or(true, |c| c != 0);

                if crashed {
                    warn!(exit_code = ?exit_code, "godot process exited with non-zero code");
                } else {
                    debug!(duration = ?duration, "godot process completed successfully");
                }

                Ok(RunResult {
                    exit_code,
                    stdout,
                    stderr,
                    duration,
                    crashed,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => {
                error!(error = %e, "failed to run godot");
                Err(e)
            }
            Err(_) => {
                warn!(timeout = ?self.config.timeout, "godot process timed out");
                Ok(RunResult {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Process timed out".to_string(),
                    duration,
                    crashed: false,
                    timed_out: true,
                })
            }
        }
    }

    pub async fn validate_project_structure(&self) -> Result<ProjectValidation> {
        let project_file = self.config.project_path.join("project.godot");
        let has_project_file = tokio::fs::metadata(&project_file).await.is_ok();

        let scenes = self.find_files_by_extension("tscn").await?;
        let scripts = self.find_files_by_extension("gd").await?;
        let resources = self.find_files_by_extension("tres").await?;

        let mut missing_refs = Vec::new();
        for scene in &scenes {
            let refs = self.check_scene_references(scene).await?;
            missing_refs.extend(refs);
        }

        Ok(ProjectValidation {
            has_project_file,
            scene_count: scenes.len(),
            script_count: scripts.len(),
            resource_count: resources.len(),
            scenes,
            scripts,
            resources,
            missing_references: missing_refs,
        })
    }

    async fn find_files_by_extension(&self, ext: &str) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![self.config.project_path.clone()];

        while let Some(dir) = stack.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    if !path.file_name().map_or(false, |n| n.to_str().map_or(false, |s| s.starts_with('.'))) {
                        stack.push(path);
                    }
                } else if path.extension().map_or(false, |e| e == ext) {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    async fn check_scene_references(&self, scene_path: &Path) -> Result<Vec<MissingReference>> {
        let content = match tokio::fs::read_to_string(scene_path).await {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()),
        };

        let mut missing = Vec::new();

        for line in content.lines() {
            if let Some(path_start) = line.find("path=\"res://") {
                let rest = &line[path_start + 6..];
                if let Some(end) = rest.find('"') {
                    let ref_path = &rest[..end];
                    let full_path = self.config.project_path.join(ref_path.trim_start_matches("res://"));
                    if !full_path.exists() {
                        missing.push(MissingReference {
                            source_file: scene_path.to_path_buf(),
                            referenced_path: ref_path.to_string(),
                        });
                    }
                }
            }
        }

        Ok(missing)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectValidation {
    pub has_project_file: bool,
    pub scene_count: usize,
    pub script_count: usize,
    pub resource_count: usize,
    pub scenes: Vec<PathBuf>,
    pub scripts: Vec<PathBuf>,
    pub resources: Vec<PathBuf>,
    pub missing_references: Vec<MissingReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingReference {
    pub source_file: PathBuf,
    pub referenced_path: String,
}
