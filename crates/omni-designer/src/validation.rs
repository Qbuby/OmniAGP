use crate::schema::{GameDesignDocument, Level};

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

pub fn validate_gdd(gdd: &GameDesignDocument) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if gdd.game_meta.title.is_empty() {
        errors.push(ValidationError {
            field: "game_meta.title".into(),
            message: "Game title is required".into(),
        });
    }

    if gdd.mechanics.is_empty() {
        errors.push(ValidationError {
            field: "mechanics".into(),
            message: "At least one mechanic must be defined".into(),
        });
    }

    if !gdd.mechanics.iter().any(|m| m.core_loop) {
        warnings.push(ValidationError {
            field: "mechanics".into(),
            message: "No mechanic is marked as core_loop".into(),
        });
    }

    if gdd.entities.is_empty() {
        errors.push(ValidationError {
            field: "entities".into(),
            message: "At least one entity must be defined".into(),
        });
    }

    if gdd.levels.is_empty() {
        errors.push(ValidationError {
            field: "levels".into(),
            message: "At least one level must be defined".into(),
        });
    }

    for level in &gdd.levels {
        validate_level(level, gdd, &mut errors);
    }

    for entity in &gdd.entities {
        if entity.sprite_ref.is_some() {
            let sprite_id = entity.sprite_ref.as_ref().unwrap();
            if !gdd.assets_spec.iter().any(|a| &a.id == sprite_id) {
                errors.push(ValidationError {
                    field: format!("entities[{}].sprite_ref", entity.id),
                    message: format!(
                        "Entity '{}' references asset '{}' which is not in assets_spec",
                        entity.name, sprite_id
                    ),
                });
            }
        } else {
            warnings.push(ValidationError {
                field: format!("entities[{}].sprite_ref", entity.id),
                message: format!("Entity '{}' has no sprite_ref — no asset will be generated", entity.name),
            });
        }
    }

    for level in &gdd.levels {
        if let Some(ref bg) = level.background_ref {
            if !gdd.assets_spec.iter().any(|a| &a.id == bg) {
                errors.push(ValidationError {
                    field: format!("levels[{}].background_ref", level.id),
                    message: format!(
                        "Level '{}' references background '{}' not in assets_spec",
                        level.name, bg
                    ),
                });
            }
        }
        if let Some(ref music) = level.music_ref {
            if !gdd.assets_spec.iter().any(|a| &a.id == music) {
                errors.push(ValidationError {
                    field: format!("levels[{}].music_ref", level.id),
                    message: format!(
                        "Level '{}' references music '{}' not in assets_spec",
                        level.name, music
                    ),
                });
            }
        }
    }

    for level in &gdd.levels {
        for entity_ref in &level.entities {
            if !gdd.entities.iter().any(|e| &e.id == entity_ref) {
                errors.push(ValidationError {
                    field: format!("levels[{}].entities", level.id),
                    message: format!(
                        "Level '{}' references entity '{}' which is not defined",
                        level.name, entity_ref
                    ),
                });
            }
        }
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

fn validate_level(level: &Level, _gdd: &GameDesignDocument, errors: &mut Vec<ValidationError>) {
    if level.win_condition.condition_type.is_empty() {
        errors.push(ValidationError {
            field: format!("levels[{}].win_condition", level.id),
            message: format!("Level '{}' has no win condition type", level.name),
        });
    }
    if level.lose_condition.condition_type.is_empty() {
        errors.push(ValidationError {
            field: format!("levels[{}].lose_condition", level.id),
            message: format!("Level '{}' has no lose condition type", level.name),
        });
    }
}
