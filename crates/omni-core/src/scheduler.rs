use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Urgent,
    Normal,
    Batch,
}

impl TaskPriority {
    pub fn weight(&self) -> u8 {
        match self {
            Self::Urgent => 10,
            Self::Normal => 5,
            Self::Batch => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCapability {
    Image2D,
    Model3D,
    Audio,
    LlmInference,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub id: Uuid,
    pub project_id: Uuid,
    pub priority: TaskPriority,
    pub capability: TaskCapability,
    pub min_vram_gb: u32,
    pub payload: serde_json::Value,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Dispatched,
    Running,
    Completed,
    Failed,
    DeadLetter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub worker_id: Uuid,
    pub status: TaskStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub gpu_minutes: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuType {
    Nvidia,
    Amd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerStatus {
    Online,
    Busy,
    Draining,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerLocation {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub gpu_type: GpuType,
    pub name: String,
    pub vram_total_mb: u32,
    pub vram_free_mb: u32,
    pub utilization_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistration {
    pub worker_id: Uuid,
    pub hostname: String,
    pub location: WorkerLocation,
    pub gpus: Vec<GpuInfo>,
    pub capabilities: Vec<TaskCapability>,
    pub labels: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    pub worker_id: Uuid,
    pub gpus: Vec<GpuInfo>,
    pub active_tasks: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerNode {
    pub id: Uuid,
    pub hostname: String,
    pub location: WorkerLocation,
    pub status: WorkerStatus,
    pub gpus: Vec<GpuInfo>,
    pub capabilities: Vec<TaskCapability>,
    pub labels: std::collections::HashMap<String, String>,
    pub active_tasks: u32,
    pub last_heartbeat: i64,
    pub registered_at: i64,
}

impl WorkerNode {
    pub fn total_vram_mb(&self) -> u32 {
        self.gpus.iter().map(|g| g.vram_total_mb).sum()
    }

    pub fn free_vram_mb(&self) -> u32 {
        self.gpus.iter().map(|g| g.vram_free_mb).sum()
    }

    pub fn load_factor(&self) -> f64 {
        let total = self.total_vram_mb() as f64;
        if total == 0.0 {
            return 1.0;
        }
        1.0 - (self.free_vram_mb() as f64 / total)
    }

    pub fn supports_capability(&self, cap: &TaskCapability) -> bool {
        self.capabilities.contains(cap) || self.capabilities.contains(&TaskCapability::General)
    }

    pub fn has_sufficient_vram(&self, required_mb: u32) -> bool {
        self.free_vram_mb() >= required_mb
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuota {
    pub user_id: Uuid,
    pub free_minutes_remaining: f64,
    pub paid_minutes_remaining: f64,
    pub total_used_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub user_id: Uuid,
    pub task_id: Uuid,
    pub worker_id: Uuid,
    pub gpu_minutes: f64,
    pub timestamp: i64,
}
