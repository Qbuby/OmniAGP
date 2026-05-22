use std::path::Path;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReport {
    pub project_name: String,
    pub description: String,
    pub platform: String,
    pub quality: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_duration_secs: f64,
    pub stages: Vec<StageReport>,
    pub token_usage: TokenUsage,
    pub assets: Vec<AssetEntry>,
    pub qa_results: QaResults,
    pub fix_records: Vec<FixRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageReport {
    pub name: String,
    pub status: String,
    pub duration_secs: f64,
    pub tokens_used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub total_prompt_tokens: u32,
    pub total_completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub name: String,
    pub asset_type: String,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QaResults {
    pub tests_run: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub crash_free_seconds: u32,
    pub issues_found: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixRecord {
    pub issue: String,
    pub fix_description: String,
    pub iteration: u32,
    pub success: bool,
}

impl GenerationReport {
    pub fn new(description: &str, platform: &str, quality: &str) -> Self {
        Self {
            project_name: extract_project_name(description),
            description: description.to_string(),
            platform: platform.to_string(),
            quality: quality.to_string(),
            started_at: Utc::now(),
            completed_at: None,
            total_duration_secs: 0.0,
            stages: Vec::new(),
            token_usage: TokenUsage::default(),
            assets: Vec::new(),
            qa_results: QaResults::default(),
            fix_records: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, name: &str, status: &str, duration_secs: f64, tokens: u32) {
        self.stages.push(StageReport {
            name: name.to_string(),
            status: status.to_string(),
            duration_secs,
            tokens_used: tokens,
        });
        self.token_usage.total_tokens += tokens;
    }

    pub fn finalize(&mut self) {
        self.completed_at = Some(Utc::now());
        if let Some(completed) = self.completed_at {
            self.total_duration_secs = (completed - self.started_at).num_milliseconds() as f64 / 1000.0;
        }
    }

    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let path = output_dir.join("generation_report.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn load(output_dir: &Path) -> Result<Self> {
        let path = output_dir.join("generation_report.json");
        if !path.exists() {
            bail!("No report found at {}", path.display());
        }
        let content = std::fs::read_to_string(&path)?;
        let report: Self = serde_json::from_str(&content)?;
        Ok(report)
    }
}

fn extract_project_name(description: &str) -> String {
    let words: Vec<&str> = description.split_whitespace().take(4).collect();
    if words.is_empty() {
        "unnamed_game".to_string()
    } else {
        words.join("_").replace(|c: char| !c.is_alphanumeric() && c != '_', "")
    }
}

pub async fn show_report(args: crate::cli::ReportArgs) -> Result<()> {
    let report = GenerationReport::load(&args.path)?;

    println!("\n  Generation Report: {}", report.project_name);
    println!("  {}", "=".repeat(50));
    println!("  Description: {}", report.description);
    println!("  Platform: {} | Quality: {}", report.platform, report.quality);
    println!(
        "  Duration: {:.1}s | Tokens: {}",
        report.total_duration_secs, report.token_usage.total_tokens
    );
    println!("\n  Stages:");
    for stage in &report.stages {
        let status_icon = if stage.status == "complete" { "✓" } else { "✗" };
        println!(
            "    {} {:<20} {:.1}s  ({} tokens)",
            status_icon, stage.name, stage.duration_secs, stage.tokens_used
        );
    }

    if !report.assets.is_empty() {
        println!("\n  Assets ({}):", report.assets.len());
        for asset in &report.assets {
            println!("    - {} [{}] ({})", asset.name, asset.asset_type, asset.path);
        }
    }

    let qa = &report.qa_results;
    println!(
        "\n  QA: {}/{} tests passed | crash-free: {}s",
        qa.tests_passed, qa.tests_run, qa.crash_free_seconds
    );

    if !report.fix_records.is_empty() {
        println!("\n  Fixes applied ({}):", report.fix_records.len());
        for fix in &report.fix_records {
            let icon = if fix.success { "✓" } else { "✗" };
            println!("    {} [iter {}] {}", icon, fix.iteration, fix.fix_description);
        }
    }

    println!();
    Ok(())
}
