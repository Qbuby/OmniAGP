use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

mod game_spec;
mod godot_project;
mod headless_qa;
mod metrics;
mod mock_services;
mod packager;
mod pipeline_runner;

use game_spec::MinimalGameSpec;
use metrics::PerfMetrics;
use pipeline_runner::SmokeTestPipeline;

#[derive(Debug, Serialize)]
struct SmokeTestReport {
    run_id: Uuid,
    timestamp: String,
    success: bool,
    stages: Vec<StageResult>,
    metrics: PerfMetrics,
    errors: Vec<String>,
    output_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct StageResult {
    name: String,
    success: bool,
    duration_ms: u64,
    details: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let run_id = Uuid::new_v4();
    let total_start = Instant::now();
    info!(run_id = %run_id, "=== OmniAGP M7 Smoke Test START ===");

    let output_dir = PathBuf::from(
        std::env::var("SMOKE_OUTPUT_DIR").unwrap_or_else(|_| format!("output/smoke-{}", run_id)),
    );
    std::fs::create_dir_all(&output_dir)?;

    let mut report = SmokeTestReport {
        run_id,
        timestamp: Utc::now().to_rfc3339(),
        success: false,
        stages: Vec::new(),
        metrics: PerfMetrics::default(),
        errors: Vec::new(),
        output_path: None,
    };

    let spec = MinimalGameSpec::default();
    info!(game = %spec.title, "target: minimal complete game");

    // Stage 1: Pipeline orchestration (GameDesigner → AssetSpec → CodeGen → Assets → Assembly)
    let stage_start = Instant::now();
    let pipeline = SmokeTestPipeline::new(&output_dir).await;
    let pipeline_result = match pipeline {
        Ok(p) => p.run_full(&spec).await,
        Err(e) => Err(e),
    };

    let (pipeline_output, stage1_success) = match pipeline_result {
        Ok(output) => {
            info!("pipeline completed successfully");
            (Some(output), true)
        }
        Err(e) => {
            let msg = format!("pipeline failed: {}", e);
            error!("{}", msg);
            report.errors.push(msg);
            (None, false)
        }
    };

    report.stages.push(StageResult {
        name: "pipeline_orchestration".into(),
        success: stage1_success,
        duration_ms: stage_start.elapsed().as_millis() as u64,
        details: pipeline_output
            .as_ref()
            .map(|o| serde_json::to_value(&o.summary).unwrap_or_default())
            .unwrap_or(serde_json::json!({"error": "pipeline failed"})),
    });
    report.metrics.pipeline_duration_ms = stage_start.elapsed().as_millis() as u64;

    if !stage1_success {
        report.metrics.total_duration_ms = total_start.elapsed().as_millis() as u64;
        write_report(&output_dir, &report)?;
        anyhow::bail!("smoke test failed at pipeline stage");
    }

    let pipeline_output = pipeline_output.unwrap();

    // Stage 2: Godot project assembly
    let stage_start = Instant::now();
    let godot_result = godot_project::assemble_godot_project(&output_dir, &pipeline_output).await;
    let stage2_success = godot_result.is_ok();
    if let Err(ref e) = godot_result {
        report.errors.push(format!("godot assembly: {}", e));
    }
    report.stages.push(StageResult {
        name: "godot_project_assembly".into(),
        success: stage2_success,
        duration_ms: stage_start.elapsed().as_millis() as u64,
        details: serde_json::json!({"assembled": stage2_success}),
    });

    if !stage2_success {
        report.metrics.total_duration_ms = total_start.elapsed().as_millis() as u64;
        write_report(&output_dir, &report)?;
        anyhow::bail!("smoke test failed at godot assembly stage");
    }

    // Stage 3: Headless QA
    let stage_start = Instant::now();
    let qa_result = headless_qa::run_headless_qa(&output_dir).await;
    let stage3_success = match &qa_result {
        Ok(r) => r.passed,
        Err(_) => false,
    };
    if let Err(ref e) = qa_result {
        report.errors.push(format!("headless QA: {}", e));
    }
    report.stages.push(StageResult {
        name: "headless_qa".into(),
        success: stage3_success,
        duration_ms: stage_start.elapsed().as_millis() as u64,
        details: qa_result
            .as_ref()
            .map(|r| serde_json::to_value(r).unwrap_or_default())
            .unwrap_or(serde_json::json!({"error": "QA failed"})),
    });

    // Stage 4: Windows executable export
    let stage_start = Instant::now();
    let export_result = packager::export_windows(&output_dir).await;
    let stage4_success = export_result.is_ok();
    let exe_path = export_result.as_ref().ok().cloned();
    if let Err(ref e) = export_result {
        report.errors.push(format!("windows export: {}", e));
    }
    report.stages.push(StageResult {
        name: "windows_export".into(),
        success: stage4_success,
        duration_ms: stage_start.elapsed().as_millis() as u64,
        details: serde_json::json!({
            "exported": stage4_success,
            "exe_path": exe_path,
        }),
    });

    // Stage 5: Performance metrics collection
    report.metrics.total_duration_ms = total_start.elapsed().as_millis() as u64;
    report.metrics.llm_tokens_used = pipeline_output.tokens_used;
    report.metrics.asset_generation_ms = pipeline_output.asset_gen_duration_ms;
    report.metrics.code_generation_ms = pipeline_output.code_gen_duration_ms;

    report.stages.push(StageResult {
        name: "performance_baseline".into(),
        success: true,
        duration_ms: 0,
        details: serde_json::to_value(&report.metrics).unwrap_or_default(),
    });

    report.success = stage1_success && stage2_success && stage3_success && stage4_success;
    report.output_path = exe_path;

    write_report(&output_dir, &report)?;

    if report.success {
        info!(
            total_ms = report.metrics.total_duration_ms,
            tokens = report.metrics.llm_tokens_used,
            "=== SMOKE TEST PASSED ==="
        );
    } else {
        error!(
            errors = report.errors.len(),
            "=== SMOKE TEST FAILED ==="
        );
    }

    if !report.success {
        std::process::exit(1);
    }

    Ok(())
}

fn write_report(output_dir: &Path, report: &SmokeTestReport) -> Result<()> {
    let report_path = output_dir.join("smoke-test-report.json");
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(&report_path, &json)?;
    info!(path = %report_path.display(), "report written");
    Ok(())
}
