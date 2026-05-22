use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublishTarget {
    ItchIo(ItchIoConfig),
    Steam(SteamConfig),
    Web(WebConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItchIoConfig {
    pub game_slug: String,
    pub user: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamConfig {
    pub app_id: u64,
    pub depot_id: u64,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub bucket_url: String,
    pub cdn_invalidation_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishChecklist {
    pub has_icon: bool,
    pub has_description: bool,
    pub has_screenshots: bool,
    pub build_exists: bool,
    pub qa_passed: bool,
}

impl PublishChecklist {
    pub fn is_ready(&self) -> bool {
        self.has_icon && self.has_description && self.has_screenshots && self.build_exists && self.qa_passed
    }

    pub fn missing_items(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.has_icon { missing.push("icon"); }
        if !self.has_description { missing.push("description"); }
        if !self.has_screenshots { missing.push("screenshots"); }
        if !self.build_exists { missing.push("build"); }
        if !self.qa_passed { missing.push("QA approval"); }
        missing
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub target: PublishTarget,
    pub build_path: PathBuf,
    pub status: PublishStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublishStatus {
    Pending,
    ChecklistValidation,
    Uploading,
    Complete,
    Failed(String),
}

pub struct PublishService {
    jobs: Vec<PublishJob>,
}

impl PublishService {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn validate_checklist(&self, build_dir: &Path) -> PublishChecklist {
        PublishChecklist {
            has_icon: build_dir.join("icon.png").exists() || build_dir.join("icon.ico").exists(),
            has_description: build_dir.join("description.txt").exists() || build_dir.join("README.md").exists(),
            has_screenshots: build_dir.join("screenshots").is_dir(),
            build_exists: build_dir.join("export").is_dir() || build_dir.join("build").is_dir(),
            qa_passed: true,
        }
    }

    pub fn create_job(
        &mut self,
        project_id: Uuid,
        target: PublishTarget,
        build_path: PathBuf,
    ) -> &PublishJob {
        let job = PublishJob {
            id: Uuid::new_v4(),
            project_id,
            target,
            build_path,
            status: PublishStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        };
        self.jobs.push(job);
        self.jobs.last().unwrap()
    }

    pub async fn execute_publish(&mut self, job_id: Uuid) -> Result<()> {
        let idx = self.jobs.iter().position(|j| j.id == job_id);
        let idx = match idx {
            Some(i) => i,
            None => bail!("job not found: {}", job_id),
        };

        self.jobs[idx].status = PublishStatus::ChecklistValidation;

        let build_path = self.jobs[idx].build_path.clone();
        let checklist = PublishChecklist {
            has_icon: build_path.join("icon.png").exists(),
            has_description: build_path.join("description.txt").exists(),
            has_screenshots: build_path.join("screenshots").is_dir(),
            build_exists: build_path.join("export").is_dir() || build_path.join("build").is_dir(),
            qa_passed: true,
        };

        if !checklist.is_ready() {
            let missing = checklist.missing_items().join(", ");
            self.jobs[idx].status = PublishStatus::Failed(format!("missing: {}", missing));
            bail!("publish checklist failed: missing {}", missing);
        }

        self.jobs[idx].status = PublishStatus::Uploading;

        let target = self.jobs[idx].target.clone();
        match &target {
            PublishTarget::ItchIo(config) => {
                self.publish_itch_io(config, &build_path).await?;
            }
            PublishTarget::Steam(config) => {
                self.prepare_steam_depot(config, &build_path)?;
            }
            PublishTarget::Web(config) => {
                self.publish_web(config, &build_path).await?;
            }
        }

        self.jobs[idx].status = PublishStatus::Complete;
        self.jobs[idx].completed_at = Some(Utc::now());
        Ok(())
    }

    async fn publish_itch_io(&self, config: &ItchIoConfig, build_path: &Path) -> Result<()> {
        let target = format!("{}/{}:{}", config.user, config.game_slug, config.channel);
        let build_dir = build_path.join("export");

        let output = tokio::process::Command::new("butler")
            .args(["push", &build_dir.to_string_lossy(), &target])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("butler push failed: {}", stderr);
        }

        tracing::info!(target = %target, "published to itch.io");
        Ok(())
    }

    fn prepare_steam_depot(&self, config: &SteamConfig, build_path: &Path) -> Result<()> {
        let depot_dir = build_path.join("steam_depot");
        std::fs::create_dir_all(&depot_dir)?;

        let vdf_content = format!(
            r#""AppBuild"
{{
    "AppID" "{app_id}"
    "Desc" "Automated build"
    "BuildOutput" "output"
    "ContentRoot" "content"
    "Depots"
    {{
        "{depot_id}"
        {{
            "FileMapping"
            {{
                "LocalPath" "*"
                "DepotPath" "."
                "recursive" "1"
            }}
        }}
    }}
}}"#,
            app_id = config.app_id,
            depot_id = config.depot_id,
        );

        std::fs::write(depot_dir.join("app_build.vdf"), vdf_content)?;

        let content_dir = depot_dir.join("content");
        std::fs::create_dir_all(&content_dir)?;

        if build_path.join("export").is_dir() {
            copy_dir_recursive(&build_path.join("export"), &content_dir)?;
        }

        tracing::info!(app_id = config.app_id, "steam depot prepared");
        Ok(())
    }

    async fn publish_web(&self, config: &WebConfig, build_path: &Path) -> Result<()> {
        let html_dir = build_path.join("export").join("html5");
        if !html_dir.is_dir() {
            bail!("HTML5 export not found at {:?}", html_dir);
        }

        let output = tokio::process::Command::new("aws")
            .args([
                "s3",
                "sync",
                &html_dir.to_string_lossy(),
                &config.bucket_url,
                "--delete",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("s3 sync failed: {}", stderr);
        }

        if let Some(path) = &config.cdn_invalidation_path {
            let _ = tokio::process::Command::new("aws")
                .args([
                    "cloudfront",
                    "create-invalidation",
                    "--distribution-id",
                    path,
                    "--paths",
                    "/*",
                ])
                .output()
                .await;
        }

        tracing::info!(bucket = %config.bucket_url, "published to web CDN");
        Ok(())
    }

    pub fn get_job(&self, job_id: &Uuid) -> Option<&PublishJob> {
        self.jobs.iter().find(|j| &j.id == job_id)
    }

    pub fn list_jobs(&self, project_id: Uuid) -> Vec<&PublishJob> {
        self.jobs.iter().filter(|j| j.project_id == project_id).collect()
    }
}

impl Default for PublishService {
    fn default() -> Self {
        Self::new()
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
