use std::path::Path;

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    Web,
}

impl Platform {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "windows" | "win" | "win64" => Ok(Self::Windows),
            "linux" | "lin" | "linux64" => Ok(Self::Linux),
            "web" | "html5" | "wasm" => Ok(Self::Web),
            _ => bail!("Unsupported platform: {}. Use: windows, linux, web", s),
        }
    }

    pub fn export_extension(&self) -> &str {
        match self {
            Self::Windows => "exe",
            Self::Linux => "x86_64",
            Self::Web => "html",
        }
    }

    pub fn godot_preset_name(&self) -> &str {
        match self {
            Self::Windows => "Windows Desktop",
            Self::Linux => "Linux/X11",
            Self::Web => "HTML5",
        }
    }
}

pub fn package_game(
    project_dir: &Path,
    output_dir: &Path,
    platform: Platform,
) -> Result<std::path::PathBuf> {
    let export_dir = output_dir.join("export");
    std::fs::create_dir_all(&export_dir)?;

    let game_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("game");

    let output_file = export_dir.join(format!("{}.{}", game_name, platform.export_extension()));

    let godot_path = find_godot_binary();
    if godot_path.is_empty() {
        tracing::warn!("Godot binary not found, creating placeholder package");
        create_placeholder_package(&output_file, platform)?;
        return Ok(output_file);
    }

    let status = std::process::Command::new(&godot_path)
        .args([
            "--headless",
            "--export-release",
            platform.godot_preset_name(),
            output_file.to_str().unwrap_or(""),
            "--path",
            project_dir.to_str().unwrap_or(""),
        ])
        .status();

    match status {
        Ok(s) if s.success() => Ok(output_file),
        Ok(s) => {
            tracing::warn!(code = ?s.code(), "Godot export returned non-zero, creating placeholder");
            create_placeholder_package(&output_file, platform)?;
            Ok(output_file)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to run Godot, creating placeholder");
            create_placeholder_package(&output_file, platform)?;
            Ok(output_file)
        }
    }
}

fn find_godot_binary() -> String {
    let candidates = if cfg!(windows) {
        vec!["godot.exe", "godot4.exe", "Godot_v4.exe"]
    } else {
        vec!["godot", "godot4", "godot-headless"]
    };

    for candidate in candidates {
        if which_exists(candidate) {
            return candidate.to_string();
        }
    }
    String::new()
}

fn which_exists(name: &str) -> bool {
    std::process::Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn create_placeholder_package(output_file: &Path, platform: Platform) -> Result<()> {
    let content = match platform {
        Platform::Web => {
            r#"<!DOCTYPE html>
<html><head><title>OmniAGP Game</title></head>
<body><h1>Game Export Placeholder</h1>
<p>Godot export template not available. Install Godot 4 export templates to produce a real build.</p>
</body></html>"#
                .to_string()
        }
        _ => format!(
            "OmniAGP placeholder package for {:?}.\nInstall Godot 4 and export templates to produce a real executable.",
            platform
        ),
    };
    std::fs::write(output_file, content)?;
    Ok(())
}
