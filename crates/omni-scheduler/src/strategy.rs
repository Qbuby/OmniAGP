use omni_core::{TaskCapability, TaskPayload, WorkerNode};
use uuid::Uuid;

pub struct SchedulingStrategy {
    affinity_cache: dashmap::DashMap<Uuid, Uuid>,
}

impl SchedulingStrategy {
    pub fn new() -> Self {
        Self {
            affinity_cache: dashmap::DashMap::new(),
        }
    }

    pub fn select_node(&self, task: &TaskPayload, nodes: &[WorkerNode]) -> Option<Uuid> {
        if nodes.is_empty() {
            return None;
        }

        let required_vram_mb = task.min_vram_gb * 1024;

        let eligible: Vec<&WorkerNode> = nodes
            .iter()
            .filter(|n| n.supports_capability(&task.capability))
            .filter(|n| n.has_sufficient_vram(required_vram_mb))
            .collect();

        if eligible.is_empty() {
            return None;
        }

        if let Some(affinity_worker) = self.affinity_cache.get(&task.project_id) {
            if let Some(node) = eligible.iter().find(|n| n.id == *affinity_worker) {
                if node.load_factor() < 0.9 {
                    return Some(node.id);
                }
            }
        }

        let mut best: Option<&WorkerNode> = None;
        let mut best_score = f64::MIN;

        for node in &eligible {
            let score = compute_score(node, task);
            if score > best_score {
                best_score = score;
                best = Some(node);
            }
        }

        if let Some(selected) = best {
            self.affinity_cache.insert(task.project_id, selected.id);
            Some(selected.id)
        } else {
            None
        }
    }
}

fn compute_score(node: &WorkerNode, task: &TaskPayload) -> f64 {
    let free_ratio = if node.total_vram_mb() > 0 {
        node.free_vram_mb() as f64 / node.total_vram_mb() as f64
    } else {
        0.0
    };

    let location_bonus = match node.location {
        omni_core::WorkerLocation::Local => 0.2,
        omni_core::WorkerLocation::Cloud => 0.0,
    };

    let capability_bonus = if node.capabilities.contains(&task.capability) {
        0.3
    } else {
        0.0
    };

    let load_penalty = node.active_tasks as f64 * 0.1;

    free_ratio + location_bonus + capability_bonus - load_penalty
}

pub fn min_vram_for_capability(cap: &TaskCapability) -> u32 {
    match cap {
        TaskCapability::Image2D => 10,
        TaskCapability::Model3D => 16,
        TaskCapability::Audio => 6,
        TaskCapability::LlmInference => 8,
        TaskCapability::General => 4,
    }
}
