use anyhow::Result;
use omni_llm::{ChatMessage, ChatRequest, ChatResponse, LlmClient, Role};

pub struct GdScriptGenerator {
    llm: LlmClient,
    model: String,
}

impl GdScriptGenerator {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }

    pub async fn generate_script(&self, description: &str, class_name: &str) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: "You are a GDScript 4.x expert. Generate clean, working GDScript code. Output ONLY the code, no markdown fences or explanations.".into(),
                },
                ChatMessage {
                    role: Role::User,
                    content: format!(
                        "Generate a GDScript class named '{}' that implements:\n{}",
                        class_name, description
                    ),
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(2048),
        };

        let response: ChatResponse = self.llm.chat(&request).await?;
        let code = response.choices[0].message.content.clone();
        Ok(Self::clean_code_output(&code))
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
}
