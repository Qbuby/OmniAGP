use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GameDesignDocument {
    pub id: Uuid,
    pub game_meta: GameMeta,
    pub mechanics: Vec<Mechanic>,
    pub entities: Vec<Entity>,
    pub levels: Vec<Level>,
    pub assets_spec: Vec<AssetSpec>,
    pub ui_spec: UiSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GameMeta {
    pub title: String,
    pub genre: Genre,
    pub sub_genres: Vec<String>,
    pub description: String,
    pub target_platform: Vec<Platform>,
    pub art_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Genre {
    Action,
    Puzzle,
    Platformer,
    Shooter,
    Rpg,
    Strategy,
    Roguelike,
    Simulation,
    Racing,
    Adventure,
    Arcade,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Linux,
    MacOs,
    Web,
    Android,
    Ios,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Mechanic {
    pub name: String,
    pub description: String,
    pub core_loop: bool,
    pub inputs: Vec<String>,
    pub outcomes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub properties: Vec<Property>,
    pub behaviors: Vec<String>,
    pub sprite_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Player,
    Enemy,
    Npc,
    Item,
    Projectile,
    Obstacle,
    Collectible,
    Boss,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Property {
    pub name: String,
    pub value_type: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Level {
    pub id: String,
    pub name: String,
    pub order: u32,
    pub description: String,
    pub difficulty: Difficulty,
    pub win_condition: WinCondition,
    pub lose_condition: LoseCondition,
    pub entities: Vec<String>,
    pub music_ref: Option<String>,
    pub background_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Progressive { start: f32, end: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WinCondition {
    pub condition_type: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoseCondition {
    pub condition_type: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetSpec {
    pub id: String,
    pub name: String,
    pub asset_type: AssetType,
    pub description: String,
    pub dimensions: Option<Dimensions>,
    pub tags: Vec<String>,
    pub referenced_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Sprite,
    SpriteSheet,
    Background,
    Tileset,
    Ui,
    Audio,
    Music,
    ParticleEffect,
    Font,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub frames: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UiSpec {
    pub screens: Vec<UiScreen>,
    pub hud: Option<HudLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UiScreen {
    pub name: String,
    pub screen_type: ScreenType,
    pub elements: Vec<UiElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScreenType {
    MainMenu,
    PauseMenu,
    Settings,
    GameOver,
    Victory,
    LevelSelect,
    Inventory,
    Dialogue,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UiElement {
    pub element_type: String,
    pub label: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HudLayout {
    pub elements: Vec<HudElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HudElement {
    pub name: String,
    pub element_type: String,
    pub position: HudPosition,
    pub data_binding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HudPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Center,
}

pub fn generate_json_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(GameDesignDocument)).unwrap()
}
