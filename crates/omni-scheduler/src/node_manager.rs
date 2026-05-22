use dashmap::DashMap;
use omni_core::{WorkerHeartbeat, WorkerNode, WorkerRegistration, WorkerStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const HEARTBEAT_TIMEOUT_SECS: i64 = 90;

pub struct NodeManager {
    nodes: DashMap<Uuid, WorkerNode>,
}

impl NodeManager {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
        }
    }

    pub fn register(&self, reg: WorkerRegistration) -> WorkerNode {
        let now = now_ts();
        let node = WorkerNode {
            id: reg.worker_id,
            hostname: reg.hostname,
            location: reg.location,
            status: WorkerStatus::Online,
            gpus: reg.gpus,
            capabilities: reg.capabilities,
            labels: reg.labels,
            active_tasks: 0,
            last_heartbeat: now,
            registered_at: now,
        };
        self.nodes.insert(node.id, node.clone());
        tracing::info!(worker_id = %reg.worker_id, "worker registered");
        node
    }

    pub fn heartbeat(&self, hb: WorkerHeartbeat) {
        if let Some(mut node) = self.nodes.get_mut(&hb.worker_id) {
            node.gpus = hb.gpus;
            node.active_tasks = hb.active_tasks;
            node.last_heartbeat = hb.timestamp;
            if node.status == WorkerStatus::Offline {
                node.status = WorkerStatus::Online;
                tracing::info!(worker_id = %hb.worker_id, "worker back online");
            }
        }
    }

    pub fn check_health(&self) {
        let now = now_ts();
        for mut entry in self.nodes.iter_mut() {
            let elapsed = now - entry.last_heartbeat;
            if elapsed > HEARTBEAT_TIMEOUT_SECS && entry.status != WorkerStatus::Offline {
                tracing::warn!(worker_id = %entry.id, elapsed_secs = elapsed, "worker marked offline");
                entry.status = WorkerStatus::Offline;
            }
        }
    }

    pub fn online_nodes(&self) -> Vec<WorkerNode> {
        self.nodes
            .iter()
            .filter(|e| matches!(e.status, WorkerStatus::Online | WorkerStatus::Busy))
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn all_nodes(&self) -> Vec<WorkerNode> {
        self.nodes.iter().map(|e| e.value().clone()).collect()
    }

    pub fn get_node(&self, id: Uuid) -> Option<WorkerNode> {
        self.nodes.get(&id).map(|e| e.value().clone())
    }

    pub fn set_status(&self, worker_id: Uuid, status: WorkerStatus) {
        if let Some(mut node) = self.nodes.get_mut(&worker_id) {
            node.status = status;
        }
    }

    pub fn remove_node(&self, worker_id: Uuid) {
        self.nodes.remove(&worker_id);
        tracing::info!(worker_id = %worker_id, "worker removed");
    }

    pub fn increment_active_tasks(&self, worker_id: Uuid) {
        if let Some(mut node) = self.nodes.get_mut(&worker_id) {
            node.active_tasks += 1;
            if node.active_tasks > 0 {
                node.status = WorkerStatus::Busy;
            }
        }
    }

    pub fn decrement_active_tasks(&self, worker_id: Uuid) {
        if let Some(mut node) = self.nodes.get_mut(&worker_id) {
            node.active_tasks = node.active_tasks.saturating_sub(1);
            if node.active_tasks == 0 && node.status == WorkerStatus::Busy {
                node.status = WorkerStatus::Online;
            }
        }
    }

    pub fn idle_nodes(&self, _idle_threshold_secs: i64) -> Vec<WorkerNode> {
        let now = now_ts();
        self.nodes
            .iter()
            .filter(|e| {
                e.active_tasks == 0
                    && e.status == WorkerStatus::Online
                    && (now - e.last_heartbeat) < HEARTBEAT_TIMEOUT_SECS
            })
            .filter(|e| {
                e.location == omni_core::WorkerLocation::Cloud
            })
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
