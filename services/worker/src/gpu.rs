use omni_core::{GpuInfo, GpuType, TaskCapability};

pub fn detect_gpus() -> Vec<GpuInfo> {
    if let Some(gpus) = detect_nvidia() {
        return gpus;
    }

    tracing::warn!("no GPU detected via nvidia-smi, using CPU fallback");
    vec![GpuInfo {
        gpu_type: GpuType::Nvidia,
        name: "CPU-only (no GPU detected)".into(),
        vram_total_mb: 0,
        vram_free_mb: 0,
        utilization_pct: 0,
    }]
}

fn detect_nvidia() -> Option<Vec<GpuInfo>> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total,memory.free,utilization.gpu", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let gpus: Vec<GpuInfo> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 4 {
                return None;
            }
            Some(GpuInfo {
                gpu_type: GpuType::Nvidia,
                name: parts[0].to_string(),
                vram_total_mb: parts[1].parse().unwrap_or(0),
                vram_free_mb: parts[2].parse().unwrap_or(0),
                utilization_pct: parts[3].parse().unwrap_or(0),
            })
        })
        .collect();

    if gpus.is_empty() { None } else { Some(gpus) }
}

pub fn infer_capabilities(gpus: &[GpuInfo]) -> Vec<TaskCapability> {
    let max_vram = gpus.iter().map(|g| g.vram_total_mb).max().unwrap_or(0);
    let mut caps = vec![TaskCapability::General];

    if max_vram >= 6 * 1024 {
        caps.push(TaskCapability::Audio);
    }
    if max_vram >= 8 * 1024 {
        caps.push(TaskCapability::LlmInference);
    }
    if max_vram >= 10 * 1024 {
        caps.push(TaskCapability::Image2D);
    }
    if max_vram >= 16 * 1024 {
        caps.push(TaskCapability::Model3D);
    }

    caps
}
