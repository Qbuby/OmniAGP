use anyhow::Result;
use async_trait::async_trait;
use omni_llm::{ChatMessage, ChatRequest, LlmClient, Role};
use tracing::info;

use crate::types::{BugReport, DegradationLevel, Patch};
use super::FixStrategy;

pub struct CodeBugStrategy {
    llm: LlmClient,
    model: String,
}

impl CodeBugStrategy {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }

    fn build_system_prompt(&self, level: DegradationLevel) -> String {
        let base = "You are a GDScript 4.x debugging expert. Analyze the bug report and generate a fix.\n\
                    Output ONLY the corrected full file content, no explanations or markdown fences.";

        match level {
            DegradationLevel::Retry => base.to_string(),
            DegradationLevel::AugmentedContext => format!(
                "{}\n\nIMPORTANT: Pay extra attention to the surrounding code context. \
                 Consider type mismatches, missing imports, incorrect signal connections, \
                 and Godot 4 API changes from Godot 3.",
                base
            ),
            DegradationLevel::TemplateFallback => format!(
                "{}\n\nThe previous fix attempts failed. Use a conservative approach: \
                 replace the problematic section with a minimal, known-working implementation. \
                 Prefer simplicity over feature completeness.",
                base
            ),
            DegradationLevel::Escalate => base.to_string(),
        }
    }
}

#[async_trait]
impl FixStrategy for CodeBugStrategy {
    fn name(&self) -> &str {
        "code_bug"
    }

    async fn generate_fix(
        &self,
        bug: &BugReport,
        level: DegradationLevel,
        previous_attempts: &[Patch],
    ) -> Result<Patch> {
        info!(bug_id = %bug.id, level = ?level, "generating code bug fix");

        let mut user_content = format!(
            "Bug: {}\nError: {}\nFile: {}\n",
            bug.description,
            bug.error_message.as_deref().unwrap_or("N/A"),
            bug.related_files.first().map(|s| s.as_str()).unwrap_or("unknown"),
        );

        if let Some(trace) = &bug.stack_trace {
            user_content.push_str(&format!("\nStack trace:\n{}\n", trace));
        }

        if let Some(source) = bug.context.get("source_code") {
            user_content.push_str(&format!("\nCurrent source:\n{}\n", source));
        }

        if level >= DegradationLevel::AugmentedContext {
            if let Some(deps) = bug.context.get("dependencies") {
                user_content.push_str(&format!("\nDependencies/imports:\n{}\n", deps));
            }
            if let Some(related) = bug.context.get("related_code") {
                user_content.push_str(&format!("\nRelated code:\n{}\n", related));
            }
        }

        if !previous_attempts.is_empty() {
            user_content.push_str("\nPrevious failed fixes:\n");
            for (i, attempt) in previous_attempts.iter().enumerate() {
                user_content.push_str(&format!("Attempt {}: {}\n", i + 1, attempt.description));
            }
            user_content.push_str("\nDo NOT repeat these approaches. Try something different.\n");
        }

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: self.build_system_prompt(level),
                },
                ChatMessage {
                    role: Role::User,
                    content: user_content,
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(4096),
        };

        let response = self.llm.chat(&request).await?;
        let fixed_code = clean_code_output(&response.choices[0].message.content);

        let file_path = bug.related_files.first()
            .cloned()
            .unwrap_or_else(|| "unknown.gd".to_string());

        let original = bug.context.get("source_code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(Patch {
            file_path,
            original,
            modified: fixed_code,
            description: format!("Code bug fix (level: {:?})", level),
        })
    }
}

fn clean_code_output(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("```") {
        let lines: Vec<&str> = trimmed.lines().collect();
        let start = 1;
        let end = if lines.last().map_or(false, |l| l.trim() == "```") {
            lines.len() - 1
        } else {
            lines.len()
        };
        lines[start..end].join("\n")
    } else {
        trimmed.to_string()
    }
}
