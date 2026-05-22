use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tracing::{info, warn};

use crate::pipeline_runner::PipelineOutput;

pub async fn assemble_godot_project(output_dir: &Path, pipeline: &PipelineOutput) -> Result<()> {
    let project_dir = output_dir.join("godot_project");
    std::fs::create_dir_all(&project_dir)?;

    // project.godot
    let project_godot = format!(
        r#"; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.

config_version=5

[application]

config/name="{title}"
run/main_scene="res://scenes/start_menu.tscn"
config/features=PackedStringArray("4.2")

[display]

window/size/viewport_width=800
window/size/viewport_height=600
window/stretch/mode="canvas_items"

[input]

move_left={{
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":65,"physical_keycode":0,"key_label":0,"unicode":0,"echo":false,"script":null)]
}}
move_right={{
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":68,"physical_keycode":0,"key_label":0,"unicode":0,"echo":false,"script":null)]
}}
jump={{
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":32,"physical_keycode":0,"key_label":0,"unicode":0,"echo":false,"script":null)]
}}
attack={{
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":74,"physical_keycode":0,"key_label":0,"unicode":0,"echo":false,"script":null)]
}}

[rendering]

renderer/rendering_method="gl_compatibility"
"#,
        title = "Dragon Slayer Mini"
    );
    std::fs::write(project_dir.join("project.godot"), project_godot)?;

    // Copy scripts
    let scenes_dir = project_dir.join("scenes");
    let entities_dir = project_dir.join("entities");
    std::fs::create_dir_all(&scenes_dir)?;
    std::fs::create_dir_all(&entities_dir)?;

    for script in &pipeline.generated_scripts {
        let rel_path = script
            .path
            .trim_start_matches("res://")
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        let dest = project_dir.join(&rel_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, &script.content)?;
    }

    // Copy scene files
    let smoke_scenes_dir = output_dir.join("scenes");
    if smoke_scenes_dir.exists() {
        for entry in std::fs::read_dir(&smoke_scenes_dir)? {
            let entry = entry?;
            let dest = scenes_dir.join(entry.file_name());
            std::fs::copy(entry.path(), dest)?;
        }
    }

    // Copy assets
    let assets_dir = project_dir.join("assets");
    std::fs::create_dir_all(&assets_dir)?;
    let smoke_assets_dir = output_dir.join("assets");
    if smoke_assets_dir.exists() {
        for entry in std::fs::read_dir(&smoke_assets_dir)? {
            let entry = entry?;
            let dest = assets_dir.join(entry.file_name());
            std::fs::copy(entry.path(), dest)?;
        }
    }

    // Generate .gdextension placeholder for future native extensions
    let export_presets = r#"[preset.0]

name="Windows Desktop"
platform="Windows Desktop"
runnable=true
dedicated_server=false
custom_features=""
export_filter="all_resources"
include_filter=""
exclude_filter=""
export_path="export/DragonSlayerMini.exe"

[preset.0.options]

custom_template/debug=""
custom_template/release=""
binary_format/embed_pck=true
"#;
    std::fs::write(project_dir.join("export_presets.cfg"), export_presets)?;

    info!(path = %project_dir.display(), "godot project assembled");
    Ok(())
}
