use anyhow::Result;
use omni_plugin::{GeneratorPlugin, Plugin, PluginContext, PluginType};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct TerrainGeneratorPlugin {
    name: String,
    version: String,
    active: bool,
    terrain_type: String,
    map_width: u32,
    map_height: u32,
    seed: Option<u64>,
}

impl TerrainGeneratorPlugin {
    pub fn new() -> Self {
        Self {
            name: "procedural-terrain-generator".into(),
            version: "1.0.0".into(),
            active: false,
            terrain_type: "island".into(),
            map_width: 128,
            map_height: 128,
            seed: None,
        }
    }
}

impl Plugin for TerrainGeneratorPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Generator
    }

    fn activate(&mut self, ctx: &PluginContext) -> Result<()> {
        if let Some(t) = ctx.config.get("terrain_type") {
            if let Some(v) = t.as_str() {
                self.terrain_type = v.to_string();
            }
        }
        if let Some(w) = ctx.config.get("map_width") {
            if let Some(v) = w.as_u64() {
                self.map_width = v as u32;
            }
        }
        if let Some(h) = ctx.config.get("map_height") {
            if let Some(v) = h.as_u64() {
                self.map_height = v as u32;
            }
        }
        if let Some(s) = ctx.config.get("seed") {
            if let Some(v) = s.as_u64() {
                self.seed = Some(v);
            }
        }
        self.active = true;
        Ok(())
    }

    fn deactivate(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }
}

impl GeneratorPlugin for TerrainGeneratorPlugin {
    fn generate(
        &self,
        output_dir: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<PathBuf>> {
        let terrain_type = params
            .get("terrain_type")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.terrain_type);

        let width = params
            .get("map_width")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.map_width as u64) as u32;

        let height = params
            .get("map_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.map_height as u64) as u32;

        let seed = params
            .get("seed")
            .and_then(|v| v.as_u64())
            .or(self.seed)
            .unwrap_or(42);

        std::fs::create_dir_all(output_dir)?;

        // Generate heightmap
        let heightmap_path = output_dir.join("heightmap.json");
        let heightmap_data = serde_json::json!({
            "type": terrain_type,
            "width": width,
            "height": height,
            "seed": seed,
            "algorithm": "perlin_noise_fbm",
            "octaves": 6,
            "persistence": 0.5,
            "lacunarity": 2.0,
        });
        std::fs::write(&heightmap_path, serde_json::to_string_pretty(&heightmap_data)?)?;

        // Generate tilemap
        let tilemap_path = output_dir.join("tilemap.json");
        let tilemap_data = serde_json::json!({
            "width": width,
            "height": height,
            "tile_size": 32,
            "layers": ["ground", "decoration", "collision"],
            "biomes": match terrain_type {
                "island" => vec!["ocean", "beach", "grass", "forest", "mountain"],
                "desert" => vec!["sand", "dunes", "oasis", "rock", "canyon"],
                "arctic" => vec!["ice", "snow", "tundra", "frozen_lake", "glacier"],
                _ => vec!["plains", "hills", "forest", "water"],
            }
        });
        std::fs::write(&tilemap_path, serde_json::to_string_pretty(&tilemap_data)?)?;

        // Generate spawn points
        let spawns_path = output_dir.join("spawn_points.json");
        let spawns_data = serde_json::json!({
            "player_spawn": {"x": width / 2, "y": height / 2},
            "enemy_spawns": [
                {"x": width / 4, "y": height / 4, "type": "patrol"},
                {"x": width * 3 / 4, "y": height / 4, "type": "guard"},
                {"x": width / 4, "y": height * 3 / 4, "type": "patrol"},
            ],
            "item_spawns": [
                {"x": width / 3, "y": height / 3, "type": "health"},
                {"x": width * 2 / 3, "y": height * 2 / 3, "type": "weapon"},
            ]
        });
        std::fs::write(&spawns_path, serde_json::to_string_pretty(&spawns_data)?)?;

        Ok(vec![heightmap_path, tilemap_path, spawns_path])
    }

    fn generator_type(&self) -> &str {
        "terrain"
    }
}

fn main() {
    let plugin = TerrainGeneratorPlugin::new();
    println!("Plugin: {} v{}", plugin.name(), plugin.version());
    println!("Type: {:?}", plugin.plugin_type());
    println!("Generator type: {}", plugin.generator_type());
}
