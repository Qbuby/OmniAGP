use crate::types::{
    TemplateAsset, TemplateCategory, TemplateManifest, TemplateParam, TemplateParamType,
    TemplateScript,
};

pub fn builtin_templates() -> Vec<TemplateManifest> {
    vec![
        platformer_2d(),
        topdown_shooter(),
        puzzle_game(),
        visual_novel(),
        idle_clicker(),
    ]
}

fn platformer_2d() -> TemplateManifest {
    TemplateManifest {
        id: "platformer-2d".into(),
        name: "2D Platformer".into(),
        description: "Classic side-scrolling platformer with jump mechanics, enemies, and collectibles".into(),
        version: "1.0.0".into(),
        author: "OmniAGP".into(),
        category: TemplateCategory::Platformer,
        engine: "Godot4".into(),
        inherits: None,
        params: vec![
            TemplateParam {
                key: "theme".into(),
                label: "Game Theme".into(),
                description: "Visual theme for the game world".into(),
                param_type: TemplateParamType::Theme,
                required: true,
                default_value: Some(serde_json::json!("forest")),
            },
            TemplateParam {
                key: "level_count".into(),
                label: "Number of Levels".into(),
                description: "How many levels to generate".into(),
                param_type: TemplateParamType::Integer { min: Some(1), max: Some(50) },
                required: true,
                default_value: Some(serde_json::json!(5)),
            },
            TemplateParam {
                key: "difficulty".into(),
                label: "Difficulty".into(),
                description: "Base difficulty level".into(),
                param_type: TemplateParamType::Select {
                    options: vec!["easy".into(), "medium".into(), "hard".into()],
                },
                required: true,
                default_value: Some(serde_json::json!("medium")),
            },
            TemplateParam {
                key: "has_double_jump".into(),
                label: "Double Jump".into(),
                description: "Enable double jump mechanic".into(),
                param_type: TemplateParamType::Boolean,
                required: false,
                default_value: Some(serde_json::json!(true)),
            },
        ],
        gdd_template: "gdd_template.json".into(),
        assets: vec![
            TemplateAsset { path: "assets/player".into(), asset_type: "sprite".into(), description: "Player character sprites".into() },
            TemplateAsset { path: "assets/tiles".into(), asset_type: "tileset".into(), description: "Platform tiles".into() },
            TemplateAsset { path: "assets/enemies".into(), asset_type: "sprite".into(), description: "Enemy sprites".into() },
        ],
        scripts: vec![
            TemplateScript { path: "scripts/player_controller.gd".into(), description: "Player movement and physics".into() },
            TemplateScript { path: "scripts/level_generator.gd".into(), description: "Procedural level generation".into() },
        ],
    }
}

fn topdown_shooter() -> TemplateManifest {
    TemplateManifest {
        id: "topdown-shooter".into(),
        name: "Top-down Shooter".into(),
        description: "Top-down twin-stick shooter with waves of enemies and power-ups".into(),
        version: "1.0.0".into(),
        author: "OmniAGP".into(),
        category: TemplateCategory::Shooter,
        engine: "Godot4".into(),
        inherits: None,
        params: vec![
            TemplateParam {
                key: "theme".into(),
                label: "Game Theme".into(),
                description: "Visual theme (sci-fi, military, fantasy)".into(),
                param_type: TemplateParamType::Theme,
                required: true,
                default_value: Some(serde_json::json!("sci-fi")),
            },
            TemplateParam {
                key: "wave_count".into(),
                label: "Number of Waves".into(),
                description: "Total enemy waves".into(),
                param_type: TemplateParamType::Integer { min: Some(5), max: Some(100) },
                required: true,
                default_value: Some(serde_json::json!(20)),
            },
            TemplateParam {
                key: "weapon_types".into(),
                label: "Weapon Variety".into(),
                description: "Number of different weapons".into(),
                param_type: TemplateParamType::Integer { min: Some(1), max: Some(10) },
                required: false,
                default_value: Some(serde_json::json!(4)),
            },
        ],
        gdd_template: "gdd_template.json".into(),
        assets: vec![
            TemplateAsset { path: "assets/player".into(), asset_type: "sprite".into(), description: "Player ship/character".into() },
            TemplateAsset { path: "assets/weapons".into(), asset_type: "sprite".into(), description: "Weapon and projectile sprites".into() },
            TemplateAsset { path: "assets/enemies".into(), asset_type: "sprite".into(), description: "Enemy sprites".into() },
        ],
        scripts: vec![
            TemplateScript { path: "scripts/player_controller.gd".into(), description: "Twin-stick movement and aiming".into() },
            TemplateScript { path: "scripts/wave_spawner.gd".into(), description: "Enemy wave management".into() },
        ],
    }
}

fn puzzle_game() -> TemplateManifest {
    TemplateManifest {
        id: "puzzle-game".into(),
        name: "Puzzle Game".into(),
        description: "Grid-based puzzle game with match mechanics and progressive difficulty".into(),
        version: "1.0.0".into(),
        author: "OmniAGP".into(),
        category: TemplateCategory::Puzzle,
        engine: "Godot4".into(),
        inherits: None,
        params: vec![
            TemplateParam {
                key: "theme".into(),
                label: "Visual Theme".into(),
                description: "Art style for puzzle pieces".into(),
                param_type: TemplateParamType::Theme,
                required: true,
                default_value: Some(serde_json::json!("gems")),
            },
            TemplateParam {
                key: "grid_size".into(),
                label: "Grid Size".into(),
                description: "Puzzle grid dimensions (NxN)".into(),
                param_type: TemplateParamType::Integer { min: Some(4), max: Some(12) },
                required: true,
                default_value: Some(serde_json::json!(8)),
            },
            TemplateParam {
                key: "level_count".into(),
                label: "Number of Levels".into(),
                description: "Total puzzle levels".into(),
                param_type: TemplateParamType::Integer { min: Some(10), max: Some(500) },
                required: true,
                default_value: Some(serde_json::json!(50)),
            },
        ],
        gdd_template: "gdd_template.json".into(),
        assets: vec![
            TemplateAsset { path: "assets/pieces".into(), asset_type: "sprite".into(), description: "Puzzle piece sprites".into() },
            TemplateAsset { path: "assets/ui".into(), asset_type: "ui".into(), description: "UI elements".into() },
        ],
        scripts: vec![
            TemplateScript { path: "scripts/grid_manager.gd".into(), description: "Grid logic and matching".into() },
            TemplateScript { path: "scripts/level_progression.gd".into(), description: "Difficulty scaling".into() },
        ],
    }
}

fn visual_novel() -> TemplateManifest {
    TemplateManifest {
        id: "visual-novel".into(),
        name: "Visual Novel".into(),
        description: "Branching narrative game with character dialogues, choices, and multiple endings".into(),
        version: "1.0.0".into(),
        author: "OmniAGP".into(),
        category: TemplateCategory::VisualNovel,
        engine: "Godot4".into(),
        inherits: None,
        params: vec![
            TemplateParam {
                key: "theme".into(),
                label: "Story Genre".into(),
                description: "Genre/setting for the story".into(),
                param_type: TemplateParamType::Theme,
                required: true,
                default_value: Some(serde_json::json!("romance")),
            },
            TemplateParam {
                key: "character_count".into(),
                label: "Number of Characters".into(),
                description: "Main characters in the story".into(),
                param_type: TemplateParamType::Integer { min: Some(2), max: Some(10) },
                required: true,
                default_value: Some(serde_json::json!(4)),
            },
            TemplateParam {
                key: "ending_count".into(),
                label: "Number of Endings".into(),
                description: "Different story endings".into(),
                param_type: TemplateParamType::Integer { min: Some(2), max: Some(8) },
                required: true,
                default_value: Some(serde_json::json!(3)),
            },
        ],
        gdd_template: "gdd_template.json".into(),
        assets: vec![
            TemplateAsset { path: "assets/characters".into(), asset_type: "sprite".into(), description: "Character portraits".into() },
            TemplateAsset { path: "assets/backgrounds".into(), asset_type: "background".into(), description: "Scene backgrounds".into() },
        ],
        scripts: vec![
            TemplateScript { path: "scripts/dialogue_system.gd".into(), description: "Dialogue and choice system".into() },
            TemplateScript { path: "scripts/story_manager.gd".into(), description: "Branching narrative logic".into() },
        ],
    }
}

fn idle_clicker() -> TemplateManifest {
    TemplateManifest {
        id: "idle-clicker".into(),
        name: "Idle/Clicker".into(),
        description: "Incremental idle game with upgrades, prestige mechanics, and offline progress".into(),
        version: "1.0.0".into(),
        author: "OmniAGP".into(),
        category: TemplateCategory::Idle,
        engine: "Godot4".into(),
        inherits: None,
        params: vec![
            TemplateParam {
                key: "theme".into(),
                label: "Game Theme".into(),
                description: "Theme for the idle game".into(),
                param_type: TemplateParamType::Theme,
                required: true,
                default_value: Some(serde_json::json!("factory")),
            },
            TemplateParam {
                key: "upgrade_tiers".into(),
                label: "Upgrade Tiers".into(),
                description: "Number of upgrade tiers".into(),
                param_type: TemplateParamType::Integer { min: Some(3), max: Some(20) },
                required: true,
                default_value: Some(serde_json::json!(8)),
            },
            TemplateParam {
                key: "has_prestige".into(),
                label: "Prestige System".into(),
                description: "Include prestige/rebirth mechanic".into(),
                param_type: TemplateParamType::Boolean,
                required: false,
                default_value: Some(serde_json::json!(true)),
            },
        ],
        gdd_template: "gdd_template.json".into(),
        assets: vec![
            TemplateAsset { path: "assets/ui".into(), asset_type: "ui".into(), description: "UI elements and icons".into() },
            TemplateAsset { path: "assets/effects".into(), asset_type: "particle".into(), description: "Click and upgrade effects".into() },
        ],
        scripts: vec![
            TemplateScript { path: "scripts/idle_engine.gd".into(), description: "Core idle/increment logic".into() },
            TemplateScript { path: "scripts/upgrade_system.gd".into(), description: "Upgrade tree management".into() },
            TemplateScript { path: "scripts/prestige.gd".into(), description: "Prestige/rebirth system".into() },
        ],
    }
}
