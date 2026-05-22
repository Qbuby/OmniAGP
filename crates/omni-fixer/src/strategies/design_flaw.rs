use anyhow::Result;
use async_trait::async_trait;
use omni_llm::{ChatMessage, ChatRequest, LlmClient, Role};
use tracing::info;

use crate::types::{BugReport, DegradationLevel, Patch};
use super::FixStrategy;

pub struct DesignFlawStrategy {
    llm: LlmClient,
    model: String,
}

impl DesignFlawStrategy {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }
}

#[async_trait]
impl FixStrategy for DesignFlawStrategy {
    fn name(&self) -> &str {
        "design_flaw"
    }

    async fn generate_fix(
        &self,
        bug: &BugReport,
        level: DegradationLevel,
        _previous_attempts: &[Patch],
    ) -> Result<Patch> {
        info!(bug_id = %bug.id, level = ?level, "generating design flaw fix");

        let parameter_name = bug.context.get("parameter")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_param");

        let current_value = bug.context.get("current_value")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A");

        match level {
            DegradationLevel::Retry | DegradationLevel::AugmentedContext => {
                let mut prompt = format!(
                    "A game design parameter is causing issues.\n\
                     Parameter: {}\nCurrent value: {}\n\
                     Problem: {}\n\n\
                     Suggest a corrected value and the code change needed. \
                     Output ONLY the corrected code section.",
                    parameter_name, current_value, bug.description,
                );

                if level == DegradationLevel::AugmentedContext {
                    if let Some(gdd) = bug.context.get("gdd_section") {
                        prompt.push_str(&format!("\n\nGDD context:\n{}", gdd));
                    }
                    if let Some(balance) = bug.context.get("balance_data") {
                        prompt.push_str(&format!("\n\nBalance data:\n{}", balance));
                    }
                }

                let request = ChatRequest {
                    model: self.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: Role::System,
                            content: "You are a game balance expert. Adjust game design parameters \
                                     to fix gameplay issues. Consider difficulty curves, player experience, \
                                     and standard game design patterns. Output ONLY the corrected code."
                                .to_string(),
                        },
                        ChatMessage {
                            role: Role::User,
                            content: prompt,
                        },
                    ],
                    temperature: Some(0.3),
                    max_tokens: Some(2048),
                };

                let response = self.llm.chat(&request).await?;
                let fixed_code = response.choices[0].message.content.trim().to_string();

                Ok(Patch {
                    file_path: bug.related_files.first().cloned().unwrap_or_default(),
                    original: bug.context.get("source_code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    modified: fixed_code,
                    description: format!("Design fix: adjusted {} from {}", parameter_name, current_value),
                })
            }
            DegradationLevel::TemplateFallback => {
                let safe_defaults = get_safe_default(parameter_name);
                let file_path = bug.related_files.first().cloned().unwrap_or_default();
                let original = bug.context.get("source_code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let modified = original.replace(current_value, &safe_defaults);

                Ok(Patch {
                    file_path,
                    original,
                    modified,
                    description: format!("Template fallback: reset {} to safe default {}", parameter_name, safe_defaults),
                })
            }
            DegradationLevel::Escalate => {
                Ok(Patch {
                    file_path: bug.related_files.first().cloned().unwrap_or_default(),
                    original: String::new(),
                    modified: String::new(),
                    description: format!("ESCALATE: Design flaw in '{}' requires human review", parameter_name),
                })
            }
        }
    }
}

fn get_safe_default(parameter_name: &str) -> String {
    match parameter_name {
        p if p.contains("speed") => "200.0".to_string(),
        p if p.contains("health") || p.contains("hp") => "100.0".to_string(),
        p if p.contains("damage") => "10.0".to_string(),
        p if p.contains("gravity") => "980.0".to_string(),
        p if p.contains("jump") => "400.0".to_string(),
        p if p.contains("spawn") || p.contains("rate") => "1.0".to_string(),
        p if p.contains("scale") => "1.0".to_string(),
        p if p.contains("difficulty") => "1.0".to_string(),
        _ => "1.0".to_string(),
    }
}
