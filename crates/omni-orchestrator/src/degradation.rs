use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradationLevel {
    Retry,
    AugmentedContext,
    TemplateFallback,
    Escalate,
}

impl DegradationLevel {
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Retry => Some(Self::AugmentedContext),
            Self::AugmentedContext => Some(Self::TemplateFallback),
            Self::TemplateFallback => Some(Self::Escalate),
            Self::Escalate => None,
        }
    }
}

impl std::fmt::Display for DegradationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retry => write!(f, "RETRY"),
            Self::AugmentedContext => write!(f, "AUGMENTED_CONTEXT"),
            Self::TemplateFallback => write!(f, "TEMPLATE_FALLBACK"),
            Self::Escalate => write!(f, "ESCALATE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub task_id: Uuid,
    pub task_type: String,
    pub error_message: String,
    pub attempt: u32,
    pub level: DegradationLevel,
    pub previous_outputs: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    RetryWithSameInput,
    RetryWithAugmentedPrompt { additional_context: String },
    UseTemplate { template_name: String },
    EscalateToHuman { reason: String },
}

pub struct ErrorDegradation {
    max_retries_per_level: u32,
    templates: std::collections::HashMap<String, serde_json::Value>,
}

impl ErrorDegradation {
    pub fn new() -> Self {
        Self {
            max_retries_per_level: 2,
            templates: Self::default_templates(),
        }
    }

    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries_per_level = max;
        self
    }

    pub fn register_template(&mut self, task_type: &str, template: serde_json::Value) {
        self.templates.insert(task_type.to_string(), template);
    }

    pub fn determine_action(&self, ctx: &ErrorContext) -> RecoveryAction {
        match ctx.level {
            DegradationLevel::Retry => {
                info!(task_id = %ctx.task_id, attempt = ctx.attempt, "retrying task");
                RecoveryAction::RetryWithSameInput
            }
            DegradationLevel::AugmentedContext => {
                let additional = self.build_augmented_context(ctx);
                warn!(task_id = %ctx.task_id, "retrying with augmented context");
                RecoveryAction::RetryWithAugmentedPrompt {
                    additional_context: additional,
                }
            }
            DegradationLevel::TemplateFallback => {
                if let Some(template_name) = self.find_template(&ctx.task_type) {
                    warn!(task_id = %ctx.task_id, template = %template_name, "falling back to template");
                    RecoveryAction::UseTemplate { template_name }
                } else {
                    RecoveryAction::EscalateToHuman {
                        reason: format!(
                            "No template available for task type '{}', error: {}",
                            ctx.task_type, ctx.error_message
                        ),
                    }
                }
            }
            DegradationLevel::Escalate => {
                warn!(task_id = %ctx.task_id, "escalating to human");
                RecoveryAction::EscalateToHuman {
                    reason: format!(
                        "All recovery attempts exhausted for task '{}'. Last error: {}",
                        ctx.task_type, ctx.error_message
                    ),
                }
            }
        }
    }

    pub fn advance_level(&self, ctx: &mut ErrorContext) -> bool {
        if ctx.attempt >= self.max_retries_per_level {
            if let Some(next) = ctx.level.next() {
                ctx.level = next;
                ctx.attempt = 0;
                true
            } else {
                false
            }
        } else {
            ctx.attempt += 1;
            true
        }
    }

    fn build_augmented_context(&self, ctx: &ErrorContext) -> String {
        let mut context = format!(
            "Previous attempt failed with: {}\n\nConstraints:\n",
            ctx.error_message
        );
        context.push_str("- Ensure output is valid JSON\n");
        context.push_str("- Keep response within token limits\n");
        context.push_str("- Follow the exact schema specified\n");

        if !ctx.previous_outputs.is_empty() {
            context.push_str("\nPrevious (failed) outputs for reference:\n");
            for (i, output) in ctx.previous_outputs.iter().enumerate().take(2) {
                let truncated = output.to_string();
                let truncated = if truncated.len() > 500 {
                    format!("{}...", &truncated[..500])
                } else {
                    truncated
                };
                context.push_str(&format!("Attempt {}: {}\n", i + 1, truncated));
            }
        }
        context
    }

    fn find_template(&self, task_type: &str) -> Option<String> {
        if self.templates.contains_key(task_type) {
            Some(task_type.to_string())
        } else {
            None
        }
    }

    pub fn get_template(&self, name: &str) -> Option<&serde_json::Value> {
        self.templates.get(name)
    }

    fn default_templates() -> std::collections::HashMap<String, serde_json::Value> {
        let mut m = std::collections::HashMap::new();
        m.insert(
            "game_design_analysis".to_string(),
            serde_json::json!({
                "genre": "platformer",
                "mechanics": ["jump", "move", "collect"],
                "scenes": ["main_menu", "level_1"],
                "entities": [
                    {"name": "player", "type": "character"},
                    {"name": "platform", "type": "static"},
                    {"name": "coin", "type": "collectible"}
                ],
                "assets_needed": {
                    "sprites": ["player.png", "platform.png", "coin.png"],
                    "audio": ["jump.ogg", "collect.ogg", "bgm.ogg"]
                }
            }),
        );
        m.insert(
            "code_generation".to_string(),
            serde_json::json!({
                "template": "minimal_platformer",
                "files": [
                    "project.godot",
                    "scenes/main.tscn",
                    "scripts/player.gd"
                ]
            }),
        );
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degradation_levels() {
        let deg = ErrorDegradation::new().with_max_retries(1);
        let mut ctx = ErrorContext {
            task_id: Uuid::new_v4(),
            task_type: "game_design_analysis".to_string(),
            error_message: "JSON parse error".to_string(),
            attempt: 0,
            level: DegradationLevel::Retry,
            previous_outputs: vec![],
        };

        let action = deg.determine_action(&ctx);
        assert!(matches!(action, RecoveryAction::RetryWithSameInput));

        ctx.attempt = 1;
        deg.advance_level(&mut ctx);
        assert_eq!(ctx.level, DegradationLevel::AugmentedContext);

        let action = deg.determine_action(&ctx);
        assert!(matches!(action, RecoveryAction::RetryWithAugmentedPrompt { .. }));
    }

    #[test]
    fn test_template_fallback() {
        let deg = ErrorDegradation::new();
        let ctx = ErrorContext {
            task_id: Uuid::new_v4(),
            task_type: "game_design_analysis".to_string(),
            error_message: "timeout".to_string(),
            attempt: 0,
            level: DegradationLevel::TemplateFallback,
            previous_outputs: vec![],
        };

        let action = deg.determine_action(&ctx);
        assert!(matches!(action, RecoveryAction::UseTemplate { .. }));
    }

    #[test]
    fn test_escalation() {
        let deg = ErrorDegradation::new();
        let ctx = ErrorContext {
            task_id: Uuid::new_v4(),
            task_type: "unknown_type".to_string(),
            error_message: "fatal".to_string(),
            attempt: 0,
            level: DegradationLevel::Escalate,
            previous_outputs: vec![],
        };

        let action = deg.determine_action(&ctx);
        assert!(matches!(action, RecoveryAction::EscalateToHuman { .. }));
    }
}
