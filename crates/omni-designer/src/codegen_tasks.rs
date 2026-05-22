use crate::schema::GameDesignDocument;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenTask {
    pub id: String,
    pub name: String,
    pub task_type: CodeGenTaskType,
    pub description: String,
    pub dependencies: Vec<String>,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeGenTaskType {
    MainScene,
    PlayerScript,
    EnemyScript,
    NpcScript,
    ItemScript,
    LevelScene,
    UiScene,
    GameManager,
    AudioManager,
    ProjectConfig,
}

pub fn decompose_gdd(gdd: &GameDesignDocument) -> Vec<CodeGenTask> {
    let mut tasks = Vec::new();

    tasks.push(CodeGenTask {
        id: "task_project_config".into(),
        name: "Project Configuration".into(),
        task_type: CodeGenTaskType::ProjectConfig,
        description: format!("Generate Godot project.godot for '{}'", gdd.game_meta.title),
        dependencies: vec![],
        context: serde_json::json!({
            "title": gdd.game_meta.title,
            "genre": gdd.game_meta.genre,
        }),
    });

    tasks.push(CodeGenTask {
        id: "task_game_manager".into(),
        name: "Game Manager".into(),
        task_type: CodeGenTaskType::GameManager,
        description: "Generate autoload game manager script (state, score, transitions)".into(),
        dependencies: vec!["task_project_config".into()],
        context: serde_json::json!({
            "levels": gdd.levels.iter().map(|l| &l.name).collect::<Vec<_>>(),
            "mechanics": gdd.mechanics.iter().map(|m| &m.name).collect::<Vec<_>>(),
        }),
    });

    for entity in &gdd.entities {
        let task_type = match entity.entity_type {
            crate::schema::EntityType::Player => CodeGenTaskType::PlayerScript,
            crate::schema::EntityType::Enemy | crate::schema::EntityType::Boss => {
                CodeGenTaskType::EnemyScript
            }
            crate::schema::EntityType::Npc => CodeGenTaskType::NpcScript,
            crate::schema::EntityType::Item
            | crate::schema::EntityType::Collectible
            | crate::schema::EntityType::Projectile => CodeGenTaskType::ItemScript,
            _ => CodeGenTaskType::ItemScript,
        };

        tasks.push(CodeGenTask {
            id: format!("task_entity_{}", entity.id),
            name: format!("{} Script", entity.name),
            task_type,
            description: format!(
                "Generate GDScript for entity '{}' ({:?})",
                entity.name, entity.entity_type
            ),
            dependencies: vec!["task_game_manager".into()],
            context: serde_json::json!({
                "entity": entity,
            }),
        });
    }

    for level in &gdd.levels {
        let entity_deps: Vec<String> = level
            .entities
            .iter()
            .map(|e| format!("task_entity_{}", e))
            .collect();

        let mut deps = vec!["task_game_manager".into()];
        deps.extend(entity_deps);

        tasks.push(CodeGenTask {
            id: format!("task_level_{}", level.id),
            name: format!("Level: {}", level.name),
            task_type: CodeGenTaskType::LevelScene,
            description: format!(
                "Generate scene and script for level '{}' (difficulty: {:?})",
                level.name, level.difficulty
            ),
            dependencies: deps,
            context: serde_json::json!({
                "level": level,
            }),
        });
    }

    tasks.push(CodeGenTask {
        id: "task_main_scene".into(),
        name: "Main Scene".into(),
        task_type: CodeGenTaskType::MainScene,
        description: "Generate main.tscn entry point scene".into(),
        dependencies: vec!["task_game_manager".into()],
        context: serde_json::json!({
            "first_level": gdd.levels.first().map(|l| &l.name),
        }),
    });

    for screen in &gdd.ui_spec.screens {
        tasks.push(CodeGenTask {
            id: format!("task_ui_{}", screen.name.to_lowercase().replace(' ', "_")),
            name: format!("UI: {}", screen.name),
            task_type: CodeGenTaskType::UiScene,
            description: format!("Generate UI scene for '{}'", screen.name),
            dependencies: vec!["task_game_manager".into()],
            context: serde_json::json!({
                "screen": screen,
            }),
        });
    }

    if gdd.assets_spec.iter().any(|a| {
        matches!(
            a.asset_type,
            crate::schema::AssetType::Audio | crate::schema::AssetType::Music
        )
    }) {
        tasks.push(CodeGenTask {
            id: "task_audio_manager".into(),
            name: "Audio Manager".into(),
            task_type: CodeGenTaskType::AudioManager,
            description: "Generate autoload audio manager for SFX and music".into(),
            dependencies: vec!["task_project_config".into()],
            context: serde_json::json!({
                "audio_assets": gdd.assets_spec.iter()
                    .filter(|a| matches!(a.asset_type, crate::schema::AssetType::Audio | crate::schema::AssetType::Music))
                    .map(|a| &a.name)
                    .collect::<Vec<_>>(),
            }),
        });
    }

    tasks
}
