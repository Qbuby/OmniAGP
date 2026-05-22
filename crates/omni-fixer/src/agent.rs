use anyhow::Result;
use chrono::Utc;
use std::time::Instant;
use tracing::{error, info, warn};

use omni_llm::LlmClient;

use crate::strategies::asset_missing::AssetMissingStrategy;
use crate::strategies::code_bug::CodeBugStrategy;
use crate::strategies::design_flaw::DesignFlawStrategy;
use crate::strategies::performance::PerformanceStrategy;
use crate::strategies::FixStrategy;
use crate::types::*;

pub struct FixerAgent {
    strategies: Vec<Box<dyn FixStrategy>>,
    max_rounds: u32,
    log: FixLog,
}

impl FixerAgent {
    pub fn new(llm: LlmClient, model: String) -> Self {
        let strategies: Vec<Box<dyn FixStrategy>> = vec![
            Box::new(CodeBugStrategy::new(
                LlmClient::new(llm.base_url().to_string(), llm.api_key().to_string()),
                model.clone(),
            )),
            Box::new(AssetMissingStrategy::new(
                LlmClient::new(llm.base_url().to_string(), llm.api_key().to_string()),
                model.clone(),
            )),
            Box::new(DesignFlawStrategy::new(
                LlmClient::new(llm.base_url().to_string(), llm.api_key().to_string()),
                model.clone(),
            )),
            Box::new(PerformanceStrategy::new(
                llm,
                model,
            )),
        ];

        Self {
            strategies,
            max_rounds: 3,
            log: FixLog::new(),
        }
    }

    pub fn with_max_rounds(mut self, rounds: u32) -> Self {
        self.max_rounds = rounds;
        self
    }

    pub fn log(&self) -> &FixLog {
        &self.log
    }

    pub async fn fix(&mut self, bug: &BugReport) -> Result<FixResult> {
        info!(bug_id = %bug.id, bug_type = ?bug.bug_type, "starting fix process");

        let strategy = self.select_strategy(bug.bug_type);
        let total_start = Instant::now();
        let mut attempts: Vec<FixAttempt> = Vec::new();
        let mut level = DegradationLevel::Retry;
        let mut previous_patches: Vec<Patch> = Vec::new();

        for round in 1..=self.max_rounds {
            info!(round, level = ?level, "fix attempt");
            let round_start = Instant::now();

            let patch_result = strategy.generate_fix(bug, level, &previous_patches).await;

            let patch = match patch_result {
                Ok(p) => p,
                Err(e) => {
                    error!(round, error = %e, "strategy failed to generate patch");
                    let attempt = FixAttempt {
                        round,
                        strategy: strategy.name().to_string(),
                        degradation_level: level,
                        patch: Patch {
                            file_path: String::new(),
                            original: String::new(),
                            modified: String::new(),
                            description: format!("Generation failed: {}", e),
                        },
                        result: FixOutcome::Failed,
                        duration_ms: round_start.elapsed().as_millis() as u64,
                        timestamp: Utc::now(),
                    };
                    attempts.push(attempt);

                    if let Some(next) = level.next() {
                        level = next;
                        continue;
                    } else {
                        break;
                    }
                }
            };

            if level == DegradationLevel::Escalate {
                warn!(bug_id = %bug.id, "escalating to human intervention");
                let attempt = FixAttempt {
                    round,
                    strategy: strategy.name().to_string(),
                    degradation_level: level,
                    patch: patch.clone(),
                    result: FixOutcome::Failed,
                    duration_ms: round_start.elapsed().as_millis() as u64,
                    timestamp: Utc::now(),
                };
                attempts.push(attempt);
                break;
            }

            let outcome = self.verify_fix(&patch).await;

            let attempt = FixAttempt {
                round,
                strategy: strategy.name().to_string(),
                degradation_level: level,
                patch: patch.clone(),
                result: outcome,
                duration_ms: round_start.elapsed().as_millis() as u64,
                timestamp: Utc::now(),
            };
            attempts.push(attempt);

            match outcome {
                FixOutcome::Success => {
                    info!(bug_id = %bug.id, round, "fix successful");
                    let result = FixResult {
                        bug_id: bug.id,
                        success: true,
                        attempts,
                        final_status: FinalStatus::Fixed,
                        total_duration_ms: total_start.elapsed().as_millis() as u64,
                    };
                    self.log.record(result.clone());
                    return Ok(result);
                }
                FixOutcome::PartialFix => {
                    info!(bug_id = %bug.id, round, "partial fix, trying next level");
                    previous_patches.push(patch);
                    if let Some(next) = level.next() {
                        level = next;
                    } else {
                        break;
                    }
                }
                FixOutcome::Failed | FixOutcome::Regression => {
                    warn!(bug_id = %bug.id, round, outcome = ?outcome, "fix failed, degrading");
                    previous_patches.push(patch);
                    if let Some(next) = level.next() {
                        level = next;
                    } else {
                        break;
                    }
                }
            }
        }

        let final_status = if level == DegradationLevel::Escalate {
            FinalStatus::Escalated {
                reason: format!("All {} fix strategies exhausted for {:?}", self.max_rounds, bug.bug_type),
            }
        } else {
            FinalStatus::MaxRetriesExceeded
        };

        let result = FixResult {
            bug_id: bug.id,
            success: false,
            attempts,
            final_status,
            total_duration_ms: total_start.elapsed().as_millis() as u64,
        };
        self.log.record(result.clone());
        Ok(result)
    }

    pub async fn fix_batch(&mut self, bugs: &[BugReport]) -> Vec<FixResult> {
        let mut results = Vec::with_capacity(bugs.len());
        for bug in bugs {
            match self.fix(bug).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!(bug_id = %bug.id, error = %e, "fatal error during fix");
                    results.push(FixResult {
                        bug_id: bug.id,
                        success: false,
                        attempts: vec![],
                        final_status: FinalStatus::Escalated {
                            reason: format!("Fatal error: {}", e),
                        },
                        total_duration_ms: 0,
                    });
                }
            }
        }
        results
    }

    fn select_strategy(&self, bug_type: BugType) -> &dyn FixStrategy {
        let idx = match bug_type {
            BugType::CodeBug => 0,
            BugType::AssetMissing => 1,
            BugType::DesignFlaw => 2,
            BugType::PerformanceIssue => 3,
        };
        self.strategies[idx].as_ref()
    }

    async fn verify_fix(&self, patch: &Patch) -> FixOutcome {
        if patch.modified.is_empty() {
            return FixOutcome::Failed;
        }

        if patch.modified == patch.original {
            return FixOutcome::Failed;
        }

        if patch.file_path.ends_with(".gd") {
            if let Some(issue) = check_gdscript_syntax(&patch.modified) {
                warn!(issue, "syntax check failed on patched code");
                return FixOutcome::Failed;
            }
        }

        FixOutcome::Success
    }
}

fn check_gdscript_syntax(code: &str) -> Option<String> {
    let mut brace_depth: i32 = 0;
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    for (line_num, line) in code.lines().enumerate() {
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                _ => {}
            }
            if brace_depth < 0 || paren_depth < 0 || bracket_depth < 0 {
                return Some(format!("Unmatched closing delimiter at line {}", line_num + 1));
            }
        }
    }

    if brace_depth != 0 {
        return Some(format!("Unmatched braces: depth {}", brace_depth));
    }
    if paren_depth != 0 {
        return Some(format!("Unmatched parentheses: depth {}", paren_depth));
    }
    if bracket_depth != 0 {
        return Some(format!("Unmatched brackets: depth {}", bracket_depth));
    }

    if code.contains("func ") && !code.contains(":") {
        return Some("Function definition missing colon".to_string());
    }

    None
}
