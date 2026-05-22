use anyhow::Result;
use omni_llm::{ChatMessage, ChatRequest, LlmClient, Role};
use tracing::info;
use uuid::Uuid;

use crate::schema::GameDesignDocument;
use crate::validation::{validate_gdd, ValidationResult};

const SYSTEM_PROMPT: &str = r#"You are a professional game designer AI. Your job is to convert natural language game descriptions into a structured Game Design Document (GDD) in JSON format.

The GDD must follow this exact schema:
{
  "id": "uuid",
  "game_meta": {
    "title": "string",
    "genre": "action|puzzle|platformer|shooter|rpg|strategy|roguelike|simulation|racing|adventure|arcade|{\"other\":\"string\"}",
    "sub_genres": ["string"],
    "description": "string",
    "target_platform": ["windows"|"linux"|"mac_os"|"web"|"android"|"ios"],
    "art_style": "string"
  },
  "mechanics": [{
    "name": "string",
    "description": "string",
    "core_loop": bool,
    "inputs": ["string"],
    "outcomes": ["string"]
  }],
  "entities": [{
    "id": "unique_snake_case_id",
    "name": "string",
    "entity_type": "player|enemy|npc|item|projectile|obstacle|collectible|boss|environment",
    "properties": [{"name": "string", "value_type": "string", "default_value": "string|null"}],
    "behaviors": ["string"],
    "sprite_ref": "asset_id or null"
  }],
  "levels": [{
    "id": "unique_snake_case_id",
    "name": "string",
    "order": number,
    "description": "string",
    "difficulty": "easy|medium|hard|{\"progressive\":{\"start\":0.0,\"end\":1.0}}",
    "win_condition": {"condition_type": "string", "description": "string", "parameters": {}},
    "lose_condition": {"condition_type": "string", "description": "string", "parameters": {}},
    "entities": ["entity_id"],
    "music_ref": "asset_id or null",
    "background_ref": "asset_id or null"
  }],
  "assets_spec": [{
    "id": "unique_snake_case_id",
    "name": "string",
    "asset_type": "sprite|sprite_sheet|background|tileset|ui|audio|music|particle_effect|font",
    "description": "string describing what to generate",
    "dimensions": {"width": number, "height": number, "frames": number|null} or null,
    "tags": ["string"],
    "referenced_by": ["entity_id or level_id"]
  }],
  "ui_spec": {
    "screens": [{
      "name": "string",
      "screen_type": "main_menu|pause_menu|settings|game_over|victory|level_select|inventory|dialogue|{\"custom\":\"string\"}",
      "elements": [{"element_type": "string", "label": "string", "action": "string|null"}]
    }],
    "hud": {
      "elements": [{
        "name": "string",
        "element_type": "string",
        "position": "top_left|top_center|top_right|bottom_left|bottom_center|bottom_right|center",
        "data_binding": "string"
      }]
    } or null
  }
}

Rules:
1. Every entity with a visual representation MUST have a sprite_ref pointing to an asset in assets_spec.
2. Every level MUST have both win_condition and lose_condition with non-empty condition_type.
3. Every asset in assets_spec must have at least one entry in referenced_by.
4. Generate unique IDs in snake_case format (e.g., "player_ship", "enemy_asteroid", "level_1").
5. Output ONLY valid JSON, no markdown fences, no explanation text.
"#;

const CLARIFICATION_PROMPT: &str = r#"Based on the game description provided, identify up to 3 critical ambiguities that would prevent creating a complete game design. Ask concise questions.

Format your response as a JSON array of strings, each being one question. If the description is clear enough, return an empty array [].

Only ask about things that are truly ambiguous and critical for the design. Do NOT ask about:
- Implementation details (those are your job)
- Obvious defaults (e.g., keyboard controls for PC games)
- Things that can be reasonably inferred
"#;

pub struct GameDesignerAgent {
    llm: LlmClient,
    model: String,
}

#[derive(Debug, Clone)]
pub enum DesignStep {
    ClarificationNeeded(Vec<String>),
    Complete(GameDesignDocument),
    ValidationFailed(GameDesignDocument, ValidationResult),
}

impl GameDesignerAgent {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }

    pub async fn analyze(&self, description: &str) -> Result<DesignStep> {
        let questions = self.generate_clarifications(description).await?;
        if !questions.is_empty() {
            return Ok(DesignStep::ClarificationNeeded(questions));
        }
        self.generate_gdd(description, &[]).await
    }

    pub async fn analyze_with_answers(
        &self,
        description: &str,
        clarifications: &[(String, String)],
    ) -> Result<DesignStep> {
        self.generate_gdd(description, clarifications).await
    }

    async fn generate_clarifications(&self, description: &str) -> Result<Vec<String>> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: CLARIFICATION_PROMPT.into(),
                },
                ChatMessage {
                    role: Role::User,
                    content: format!("Game description: {}", description),
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(512),
        };

        let response = self.llm.chat(&request).await?;
        let content = &response.choices[0].message.content;
        let questions: Vec<String> = serde_json::from_str(content.trim()).unwrap_or_default();
        Ok(questions.into_iter().take(3).collect())
    }

    async fn generate_gdd(
        &self,
        description: &str,
        clarifications: &[(String, String)],
    ) -> Result<DesignStep> {
        let mut user_content = format!("Create a complete GDD for this game:\n\n{}", description);

        if !clarifications.is_empty() {
            user_content.push_str("\n\nAdditional details:\n");
            for (q, a) in clarifications {
                user_content.push_str(&format!("Q: {}\nA: {}\n\n", q, a));
            }
        }

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: SYSTEM_PROMPT.into(),
                },
                ChatMessage {
                    role: Role::User,
                    content: user_content,
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(8192),
        };

        let response = self.llm.chat(&request).await?;
        let content = &response.choices[0].message.content;

        let json_str = extract_json(content);
        let mut gdd: GameDesignDocument = serde_json::from_str(json_str)?;
        gdd.id = Uuid::new_v4();

        info!(title = %gdd.game_meta.title, entities = gdd.entities.len(), levels = gdd.levels.len(), "GDD generated");

        let validation = validate_gdd(&gdd);
        if validation.valid {
            Ok(DesignStep::Complete(gdd))
        } else {
            let repaired = self.attempt_repair(&gdd, &validation, description).await?;
            let revalidation = validate_gdd(&repaired);
            if revalidation.valid {
                Ok(DesignStep::Complete(repaired))
            } else {
                Ok(DesignStep::ValidationFailed(repaired, revalidation))
            }
        }
    }

    async fn attempt_repair(
        &self,
        gdd: &GameDesignDocument,
        validation: &ValidationResult,
        _original_description: &str,
    ) -> Result<GameDesignDocument> {
        let error_summary: Vec<String> = validation
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: format!(
                        "{}\n\nYou are repairing an existing GDD that has validation errors. Fix the errors while preserving the design intent. Output the complete corrected GDD as JSON.",
                        SYSTEM_PROMPT
                    ),
                },
                ChatMessage {
                    role: Role::User,
                    content: format!(
                        "Fix these validation errors:\n{}\n\nCurrent GDD:\n{}",
                        error_summary.join("\n"),
                        serde_json::to_string_pretty(gdd)?
                    ),
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(8192),
        };

        let response = self.llm.chat(&request).await?;
        let content = &response.choices[0].message.content;
        let json_str = extract_json(content);
        let mut repaired: GameDesignDocument = serde_json::from_str(json_str)?;
        repaired.id = gdd.id;
        Ok(repaired)
    }
}

fn extract_json(content: &str) -> &str {
    let trimmed = content.trim();
    if trimmed.starts_with('{') {
        return trimmed;
    }
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(newline) = after.find('\n') {
            let after_lang = &after[newline..];
            if let Some(end) = after_lang.find("```") {
                return after_lang[..end].trim();
            }
        }
    }
    if let Some(start) = trimmed.find('{') {
        let bytes = trimmed.as_bytes();
        let mut depth = 0;
        for (i, &b) in bytes[start..].iter().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &trimmed[start..start + i + 1];
                    }
                }
                _ => {}
            }
        }
    }
    trimmed
}
