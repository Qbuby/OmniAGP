use anyhow::Result;
use omni_llm::{ChatMessage, ChatRequest, ChatResponse, LlmClient, Role};
use omni_rag::RagRetriever;
use tracing::info;

pub struct GdScriptGenerator {
    llm: LlmClient,
    model: String,
    rag: Option<RagRetriever>,
}

impl GdScriptGenerator {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self {
            llm,
            model,
            rag: None,
        }
    }

    pub fn with_rag(mut self, rag: RagRetriever) -> Self {
        self.rag = Some(rag);
        self
    }

    pub async fn generate_script(&self, description: &str, class_name: &str) -> Result<String> {
        let mut system_content = GDSCRIPT_SYSTEM_PROMPT.to_string();

        if let Some(ref rag) = self.rag {
            let retrieval = rag.retrieve(description).await?;

            if let Some(ref template_code) = retrieval.matched_template {
                info!(class_name = class_name, "using template match");
                let customized = self.customize_template(template_code, description, class_name).await?;
                return Ok(customized);
            }

            let context = rag.build_context_prompt(&retrieval);
            if !context.is_empty() {
                system_content.push_str("\n\n# Retrieved Context\n");
                system_content.push_str(&context);
            }
        }

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: system_content,
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
            max_tokens: Some(4096),
        };

        let response: ChatResponse = self.llm.chat(&request).await?;
        let code = response.choices[0].message.content.clone();
        Ok(Self::clean_code_output(&code))
    }

    async fn customize_template(
        &self,
        template: &str,
        description: &str,
        class_name: &str,
    ) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: format!(
                        "{}\n\nYou have a template to work from. Customize it to match the user's requirements. \
                         Keep the template's structure but adapt parameters, add features, or modify logic as needed. \
                         Output ONLY the final GDScript code.",
                        GDSCRIPT_SYSTEM_PROMPT
                    ),
                },
                ChatMessage {
                    role: Role::User,
                    content: format!(
                        "Template:\n```gdscript\n{}\n```\n\nCustomize this template for a class named '{}' that implements:\n{}",
                        template, class_name, description
                    ),
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(4096),
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

const GDSCRIPT_SYSTEM_PROMPT: &str = r#"You are a GDScript 4.x expert. Generate clean, working GDScript code for Godot 4.

Key rules:
- Use static typing everywhere (var x: Type, func foo() -> Type)
- Use @export for configurable properties
- Use signals for communication between nodes
- Prefer composition over inheritance
- Use snake_case for functions/variables, PascalCase for classes
- Use StringName (&"name") for frequently compared strings
- Always call move_and_slide() for CharacterBody2D movement
- Use @onready for node references: @onready var node: Type = %UniqueName
- Godot 4 uses `func _ready()`, not `func _init()` for node setup
- Use `super()` not `.` for parent method calls
- match statement (not switch/case)
- Type annotations on signal parameters: signal foo(bar: int)
- Array typing: Array[Type]
- No semicolons at end of lines

Output ONLY the code, no markdown fences or explanations."#;
