use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

use crate::bug_report::{BugCategory, BugReport, Severity};
use crate::runner::GodotRunner;
use super::{QaTest, TestResult, TestType, TestVerdict};

pub struct PerformanceTest {
    pub project_id: Uuid,
    pub min_fps: f64,
    pub max_memory_mb: f64,
    pub sample_duration: Duration,
}

impl PerformanceTest {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project_id,
            min_fps: 30.0,
            max_memory_mb: 512.0,
            sample_duration: Duration::from_secs(10),
        }
    }

    fn generate_perf_script(&self) -> String {
        format!(
            r#"extends SceneTree

var frame_count = 0
var start_time = 0.0
var fps_samples = []
var memory_samples = []
var sample_duration = {duration:.1}

func _init():
    print("PERF_TEST_START")
    start_time = Time.get_unix_time_from_system()

func _process(delta):
    frame_count += 1
    var elapsed = Time.get_unix_time_from_system() - start_time

    if int(elapsed) > fps_samples.size():
        var current_fps = Engine.get_frames_per_second()
        fps_samples.append(current_fps)
        var mem = OS.get_static_memory_usage() / 1048576.0
        memory_samples.append(mem)
        print("PERF_SAMPLE: fps=" + str(current_fps) + " mem_mb=" + str(snapped(mem, 0.1)))

    if elapsed >= sample_duration:
        _report_results()
        quit(0)

func _report_results():
    var avg_fps = 0.0
    for fps in fps_samples:
        avg_fps += fps
    if fps_samples.size() > 0:
        avg_fps /= fps_samples.size()

    var max_mem = 0.0
    for mem in memory_samples:
        if mem > max_mem:
            max_mem = mem

    var min_fps_val = 9999.0
    for fps in fps_samples:
        if fps < min_fps_val:
            min_fps_val = fps

    print("PERF_RESULT: avg_fps=" + str(snapped(avg_fps, 0.1)) + " min_fps=" + str(snapped(min_fps_val, 0.1)) + " max_mem_mb=" + str(snapped(max_mem, 0.1)))
    print("PERF_TEST_COMPLETE")
"#,
            duration = self.sample_duration.as_secs_f64(),
        )
    }
}

#[async_trait]
impl QaTest for PerformanceTest {
    fn name(&self) -> &str {
        "performance_test"
    }

    fn test_type(&self) -> TestType {
        TestType::Performance
    }

    async fn execute(&self, runner: &GodotRunner) -> Result<TestResult> {
        info!(
            "running performance test: sampling for {:?}, min_fps={}, max_mem={}MB",
            self.sample_duration, self.min_fps, self.max_memory_mb
        );
        let start = std::time::Instant::now();

        let script_content = self.generate_perf_script();
        let script_path = runner.config.project_path.join("_qa_perf_test.gd");
        tokio::fs::write(&script_path, &script_content).await?;

        let result = runner.run_with_script(&script_path).await?;

        let _ = tokio::fs::remove_file(&script_path).await;

        let duration = start.elapsed();
        let mut bugs = Vec::new();

        if result.crashed {
            bugs.push(
                BugReport::new(
                    self.project_id,
                    BugCategory::PerformanceIssue,
                    Severity::Critical,
                    "Game crashed during performance test".to_string(),
                    "Possible out-of-memory or infinite loop".to_string(),
                    self.name().to_string(),
                )
                .with_log(result.stderr.clone()),
            );

            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Fail,
                duration,
                message: "Game crashed during performance sampling".to_string(),
                details: serde_json::json!({"crashed": true}),
                bugs,
            });
        }

        let mut avg_fps = 0.0;
        let mut min_fps = 0.0;
        let mut max_mem_mb = 0.0;
        let mut has_result = false;

        for line in result.stdout.lines() {
            if line.starts_with("PERF_RESULT:") {
                has_result = true;
                for part in line.split_whitespace() {
                    if let Some(val) = part.strip_prefix("avg_fps=") {
                        avg_fps = val.parse().unwrap_or(0.0);
                    } else if let Some(val) = part.strip_prefix("min_fps=") {
                        min_fps = val.parse().unwrap_or(0.0);
                    } else if let Some(val) = part.strip_prefix("max_mem_mb=") {
                        max_mem_mb = val.parse().unwrap_or(0.0);
                    }
                }
            }
        }

        if !has_result {
            return Ok(TestResult {
                test_name: self.name().to_string(),
                test_type: self.test_type(),
                verdict: TestVerdict::Warning,
                duration,
                message: "Performance test did not produce results".to_string(),
                details: serde_json::json!({"stdout": result.stdout}),
                bugs,
            });
        }

        if min_fps < self.min_fps {
            bugs.push(BugReport::new(
                self.project_id,
                BugCategory::PerformanceIssue,
                Severity::High,
                format!("FPS below threshold: {min_fps:.1} < {:.1}", self.min_fps),
                format!("Minimum FPS dropped to {min_fps:.1} (avg: {avg_fps:.1}). Target: {:.1}+", self.min_fps),
                self.name().to_string(),
            ));
        }

        if max_mem_mb > self.max_memory_mb {
            bugs.push(BugReport::new(
                self.project_id,
                BugCategory::PerformanceIssue,
                Severity::High,
                format!("Memory usage exceeds limit: {max_mem_mb:.1}MB > {:.1}MB", self.max_memory_mb),
                "Possible memory leak or excessive resource loading".to_string(),
                self.name().to_string(),
            ));
        }

        let verdict = if !bugs.is_empty() {
            TestVerdict::Fail
        } else {
            TestVerdict::Pass
        };

        Ok(TestResult {
            test_name: self.name().to_string(),
            test_type: self.test_type(),
            verdict,
            duration,
            message: format!("avg_fps={avg_fps:.1} min_fps={min_fps:.1} max_mem={max_mem_mb:.1}MB"),
            details: serde_json::json!({
                "avg_fps": avg_fps,
                "min_fps": min_fps,
                "max_memory_mb": max_mem_mb,
                "thresholds": {
                    "min_fps": self.min_fps,
                    "max_memory_mb": self.max_memory_mb,
                },
            }),
            bugs,
        })
    }
}
