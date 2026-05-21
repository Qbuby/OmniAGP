use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MinimalGameSpec {
    pub title: String,
    pub description: String,
    pub scenes: Vec<SceneRequirement>,
    pub entities: Vec<EntityRequirement>,
    pub assets: AssetRequirements,
}

#[derive(Debug, Clone, Serialize)]
pub struct SceneRequirement {
    pub name: String,
    pub scene_type: SceneType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum SceneType {
    Menu,
    Gameplay,
    Boss,
    EndScreen,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityRequirement {
    pub name: String,
    pub entity_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetRequirements {
    pub sprites: Vec<SpriteReq>,
    pub tiles: Vec<TileReq>,
    pub audio: Vec<AudioReq>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpriteReq {
    pub name: String,
    pub description: String,
    pub size: (u32, u32),
    pub animations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TileReq {
    pub name: String,
    pub description: String,
    pub tile_size: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioReq {
    pub name: String,
    pub description: String,
    pub audio_type: String,
    pub duration_sec: f64,
}

impl Default for MinimalGameSpec {
    fn default() -> Self {
        Self {
            title: "Dragon Slayer Mini".into(),
            description: "A minimal 2D action game: player navigates one level, defeats a boss dragon, and wins.".into(),
            scenes: vec![
                SceneRequirement {
                    name: "start_menu".into(),
                    scene_type: SceneType::Menu,
                    description: "Title screen with 'Start Game' button and game title".into(),
                },
                SceneRequirement {
                    name: "level_1".into(),
                    scene_type: SceneType::Gameplay,
                    description: "Side-scrolling level with platforms, enemies, and collectibles leading to boss area".into(),
                },
                SceneRequirement {
                    name: "boss_fight".into(),
                    scene_type: SceneType::Boss,
                    description: "Boss arena where player fights the dragon boss".into(),
                },
                SceneRequirement {
                    name: "victory_screen".into(),
                    scene_type: SceneType::EndScreen,
                    description: "Victory screen showing 'You Win!' with return to menu option".into(),
                },
            ],
            entities: vec![
                EntityRequirement {
                    name: "player".into(),
                    entity_type: "character".into(),
                    description: "Knight character with walk, jump, attack animations".into(),
                },
                EntityRequirement {
                    name: "slime_enemy".into(),
                    entity_type: "enemy".into(),
                    description: "Basic slime enemy that patrols and damages player on contact".into(),
                },
                EntityRequirement {
                    name: "dragon_boss".into(),
                    entity_type: "boss".into(),
                    description: "Dragon boss with fire breath attack and charge attack".into(),
                },
            ],
            assets: AssetRequirements {
                sprites: vec![
                    SpriteReq {
                        name: "player_knight".into(),
                        description: "Pixel art knight character, 4-directional".into(),
                        size: (32, 32),
                        animations: vec!["idle".into(), "walk".into(), "jump".into(), "attack".into()],
                    },
                    SpriteReq {
                        name: "slime".into(),
                        description: "Green slime enemy, bouncing animation".into(),
                        size: (16, 16),
                        animations: vec!["idle".into(), "move".into(), "death".into()],
                    },
                    SpriteReq {
                        name: "dragon".into(),
                        description: "Red dragon boss, large sprite".into(),
                        size: (64, 64),
                        animations: vec!["idle".into(), "attack".into(), "fire_breath".into(), "death".into()],
                    },
                ],
                tiles: vec![
                    TileReq {
                        name: "dungeon_tileset".into(),
                        description: "Stone dungeon tileset with floor, walls, platforms".into(),
                        tile_size: 16,
                    },
                ],
                audio: vec![
                    AudioReq {
                        name: "bgm_level".into(),
                        description: "Adventurous chiptune background music for dungeon level".into(),
                        audio_type: "bgm".into(),
                        duration_sec: 30.0,
                    },
                    AudioReq {
                        name: "bgm_boss".into(),
                        description: "Intense battle music for dragon boss fight".into(),
                        audio_type: "bgm".into(),
                        duration_sec: 30.0,
                    },
                    AudioReq {
                        name: "sfx_jump".into(),
                        description: "Player jump sound effect".into(),
                        audio_type: "sfx".into(),
                        duration_sec: 0.5,
                    },
                    AudioReq {
                        name: "sfx_attack".into(),
                        description: "Sword slash sound effect".into(),
                        audio_type: "sfx".into(),
                        duration_sec: 0.3,
                    },
                    AudioReq {
                        name: "sfx_dragon_roar".into(),
                        description: "Dragon roar sound for boss entrance".into(),
                        audio_type: "sfx".into(),
                        duration_sec: 1.5,
                    },
                ],
            },
        }
    }
}

impl std::fmt::Display for MinimalGameSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}
