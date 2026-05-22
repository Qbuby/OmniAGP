#[cfg(test)]
mod tests {
    use omni_designer::schema::*;
    use omni_designer::validation::validate_gdd;
    use omni_designer::codegen_tasks::decompose_gdd;
    use uuid::Uuid;

    fn make_minimal_gdd() -> GameDesignDocument {
        GameDesignDocument {
            id: Uuid::new_v4(),
            game_meta: GameMeta {
                title: "Test Snake Game".into(),
                genre: Genre::Arcade,
                sub_genres: vec![],
                description: "A classic snake game".into(),
                target_platform: vec![Platform::Web],
                art_style: "pixel art".into(),
            },
            mechanics: vec![Mechanic {
                name: "Movement".into(),
                description: "Snake moves in 4 directions".into(),
                core_loop: true,
                inputs: vec!["arrow_keys".into()],
                outcomes: vec!["position_change".into()],
            }],
            entities: vec![
                Entity {
                    id: "player_snake".into(),
                    name: "Snake".into(),
                    entity_type: EntityType::Player,
                    properties: vec![Property {
                        name: "length".into(),
                        value_type: "int".into(),
                        default_value: Some("3".into()),
                    }],
                    behaviors: vec!["move".into(), "grow".into()],
                    sprite_ref: Some("sprite_snake".into()),
                },
                Entity {
                    id: "food_apple".into(),
                    name: "Apple".into(),
                    entity_type: EntityType::Collectible,
                    properties: vec![],
                    behaviors: vec!["spawn_random".into()],
                    sprite_ref: Some("sprite_apple".into()),
                },
            ],
            levels: vec![Level {
                id: "level_1".into(),
                name: "Classic Mode".into(),
                order: 1,
                description: "Standard snake gameplay".into(),
                difficulty: Difficulty::Progressive { start: 0.2, end: 1.0 },
                win_condition: WinCondition {
                    condition_type: "score_threshold".into(),
                    description: "Reach 100 points".into(),
                    parameters: serde_json::json!({"target_score": 100}),
                },
                lose_condition: LoseCondition {
                    condition_type: "collision".into(),
                    description: "Snake hits wall or itself".into(),
                    parameters: serde_json::json!({"targets": ["wall", "self"]}),
                },
                entities: vec!["player_snake".into(), "food_apple".into()],
                music_ref: Some("music_gameplay".into()),
                background_ref: Some("bg_grid".into()),
            }],
            assets_spec: vec![
                AssetSpec {
                    id: "sprite_snake".into(),
                    name: "Snake Sprite".into(),
                    asset_type: AssetType::SpriteSheet,
                    description: "Green pixel snake segments".into(),
                    dimensions: Some(Dimensions { width: 16, height: 16, frames: Some(4) }),
                    tags: vec!["player".into(), "animated".into()],
                    referenced_by: vec!["player_snake".into()],
                },
                AssetSpec {
                    id: "sprite_apple".into(),
                    name: "Apple Sprite".into(),
                    asset_type: AssetType::Sprite,
                    description: "Red pixel apple".into(),
                    dimensions: Some(Dimensions { width: 16, height: 16, frames: None }),
                    tags: vec!["food".into()],
                    referenced_by: vec!["food_apple".into()],
                },
                AssetSpec {
                    id: "music_gameplay".into(),
                    name: "Gameplay Music".into(),
                    asset_type: AssetType::Music,
                    description: "Upbeat chiptune loop".into(),
                    dimensions: None,
                    tags: vec!["music".into(), "loop".into()],
                    referenced_by: vec!["level_1".into()],
                },
                AssetSpec {
                    id: "bg_grid".into(),
                    name: "Grid Background".into(),
                    asset_type: AssetType::Background,
                    description: "Dark grid background".into(),
                    dimensions: Some(Dimensions { width: 640, height: 480, frames: None }),
                    tags: vec!["background".into()],
                    referenced_by: vec!["level_1".into()],
                },
            ],
            ui_spec: UiSpec {
                screens: vec![
                    UiScreen {
                        name: "Main Menu".into(),
                        screen_type: ScreenType::MainMenu,
                        elements: vec![
                            UiElement { element_type: "button".into(), label: "Play".into(), action: Some("start_game".into()) },
                            UiElement { element_type: "button".into(), label: "Quit".into(), action: Some("quit".into()) },
                        ],
                    },
                    UiScreen {
                        name: "Game Over".into(),
                        screen_type: ScreenType::GameOver,
                        elements: vec![
                            UiElement { element_type: "label".into(), label: "Game Over".into(), action: None },
                            UiElement { element_type: "button".into(), label: "Retry".into(), action: Some("restart".into()) },
                        ],
                    },
                ],
                hud: Some(HudLayout {
                    elements: vec![HudElement {
                        name: "Score".into(),
                        element_type: "label".into(),
                        position: HudPosition::TopRight,
                        data_binding: "game_manager.score".into(),
                    }],
                }),
            },
        }
    }

    #[test]
    fn test_valid_gdd_passes_validation() {
        let gdd = make_minimal_gdd();
        let result = validate_gdd(&gdd);
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_missing_title_fails() {
        let mut gdd = make_minimal_gdd();
        gdd.game_meta.title = String::new();
        let result = validate_gdd(&gdd);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "game_meta.title"));
    }

    #[test]
    fn test_missing_asset_ref_fails() {
        let mut gdd = make_minimal_gdd();
        gdd.entities[0].sprite_ref = Some("nonexistent_sprite".into());
        let result = validate_gdd(&gdd);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("nonexistent_sprite")));
    }

    #[test]
    fn test_missing_win_condition_fails() {
        let mut gdd = make_minimal_gdd();
        gdd.levels[0].win_condition.condition_type = String::new();
        let result = validate_gdd(&gdd);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field.contains("win_condition")));
    }

    #[test]
    fn test_entity_not_in_level_warns() {
        let mut gdd = make_minimal_gdd();
        gdd.levels[0].entities.push("nonexistent_entity".into());
        let result = validate_gdd(&gdd);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("nonexistent_entity")));
    }

    #[test]
    fn test_decompose_produces_tasks() {
        let gdd = make_minimal_gdd();
        let tasks = decompose_gdd(&gdd);
        assert!(!tasks.is_empty());

        let task_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert!(task_ids.contains(&"task_project_config"));
        assert!(task_ids.contains(&"task_game_manager"));
        assert!(task_ids.contains(&"task_main_scene"));
        assert!(task_ids.contains(&"task_entity_player_snake"));
        assert!(task_ids.contains(&"task_entity_food_apple"));
        assert!(task_ids.contains(&"task_level_level_1"));
        assert!(task_ids.contains(&"task_audio_manager"));
    }

    #[test]
    fn test_decompose_respects_dependencies() {
        let gdd = make_minimal_gdd();
        let tasks = decompose_gdd(&gdd);

        let game_manager = tasks.iter().find(|t| t.id == "task_game_manager").unwrap();
        assert!(game_manager.dependencies.contains(&"task_project_config".to_string()));

        let level_task = tasks.iter().find(|t| t.id == "task_level_level_1").unwrap();
        assert!(level_task.dependencies.contains(&"task_game_manager".to_string()));
        assert!(level_task.dependencies.contains(&"task_entity_player_snake".to_string()));
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = omni_designer::generate_json_schema();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("$schema") || obj.contains_key("title") || obj.contains_key("type"));
    }

    #[test]
    fn test_gdd_serialization_roundtrip() {
        let gdd = make_minimal_gdd();
        let json = serde_json::to_string_pretty(&gdd).unwrap();
        let deserialized: GameDesignDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.game_meta.title, gdd.game_meta.title);
        assert_eq!(deserialized.entities.len(), gdd.entities.len());
        assert_eq!(deserialized.levels.len(), gdd.levels.len());
    }
}
