use std::time::Duration;

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct PipelineProgress {
    #[allow(dead_code)]
    multi: MultiProgress,
    stages: Vec<StageProgress>,
}

struct StageProgress {
    #[allow(dead_code)]
    name: String,
    bar: ProgressBar,
}

impl PipelineProgress {
    pub fn new(stage_names: &[&str]) -> Self {
        let multi = MultiProgress::new();
        let stages = stage_names
            .iter()
            .map(|name| {
                let bar = multi.add(ProgressBar::new(100));
                bar.set_style(
                    ProgressStyle::with_template(
                        "  {prefix:.bold.dim} [{bar:30.cyan/blue}] {msg}",
                    )
                    .unwrap()
                    .progress_chars("━━╸━"),
                );
                bar.set_prefix(format!("{:<20}", *name));
                bar.set_message(style("pending").dim().to_string());
                StageProgress {
                    name: name.to_string(),
                    bar,
                }
            })
            .collect();

        Self { multi, stages }
    }

    pub fn start_stage(&self, index: usize) {
        if let Some(stage) = self.stages.get(index) {
            stage.bar.set_message(style("running...").yellow().to_string());
            stage.bar.enable_steady_tick(Duration::from_millis(120));
        }
    }

    pub fn update_stage(&self, index: usize, progress: u64, msg: &str) {
        if let Some(stage) = self.stages.get(index) {
            stage.bar.set_position(progress);
            stage.bar.set_message(style(msg).yellow().to_string());
        }
    }

    pub fn complete_stage(&self, index: usize, duration: Duration) {
        if let Some(stage) = self.stages.get(index) {
            stage.bar.set_position(100);
            stage.bar.set_message(
                style(format!("done ({:.1}s)", duration.as_secs_f64()))
                    .green()
                    .to_string(),
            );
            stage.bar.finish();
        }
    }

    pub fn fail_stage(&self, index: usize, error: &str) {
        if let Some(stage) = self.stages.get(index) {
            stage.bar.set_message(style(format!("FAILED: {}", error)).red().to_string());
            stage.bar.abandon();
        }
    }

    pub fn print_header(description: &str, platform: &str, quality: &str) {
        println!();
        println!(
            "  {} {}",
            style("omnigp").bold().cyan(),
            style("game generation pipeline").dim()
        );
        println!("  {} {}", style("Description:").bold(), description);
        println!(
            "  {} {} | {} {}",
            style("Platform:").bold(),
            platform,
            style("Quality:").bold(),
            quality
        );
        println!();
    }

    pub fn print_summary(total_duration: Duration, output_path: &str) {
        println!();
        println!(
            "  {} Generated in {:.1}s",
            style("✓").green().bold(),
            total_duration.as_secs_f64()
        );
        println!("  {} {}", style("Output:").bold(), output_path);
        println!();
    }
}
