use crate::queue::TaskQueue;
use crate::node_manager::NodeManager;
use omni_core::WorkerLocation;
use std::sync::Arc;

pub struct AutoscaleConfig {
    pub queue_depth_threshold: u64,
    pub wait_time_threshold_secs: u64,
    pub idle_timeout_secs: i64,
    pub max_cloud_nodes: usize,
    pub min_cloud_nodes: usize,
}

impl Default for AutoscaleConfig {
    fn default() -> Self {
        Self {
            queue_depth_threshold: 10,
            wait_time_threshold_secs: 300,
            idle_timeout_secs: 900,
            max_cloud_nodes: 10,
            min_cloud_nodes: 0,
        }
    }
}

pub struct Autoscaler {
    queue: Arc<TaskQueue>,
    nodes: Arc<NodeManager>,
    config: AutoscaleConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleDecision {
    ScaleUp(usize),
    ScaleDown(usize),
    NoChange,
}

impl Autoscaler {
    pub fn new(queue: Arc<TaskQueue>, nodes: Arc<NodeManager>) -> Self {
        Self {
            queue,
            nodes,
            config: AutoscaleConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AutoscaleConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn evaluate(&self) -> ScaleDecision {
        let depth = self.queue.queue_depth().await.unwrap_or(0);
        let idle = self.nodes.idle_nodes(self.config.idle_timeout_secs);
        let cloud_nodes: Vec<_> = self.nodes.online_nodes()
            .into_iter()
            .filter(|n| n.location == WorkerLocation::Cloud)
            .collect();
        let cloud_count = cloud_nodes.len();

        if depth > self.config.queue_depth_threshold && cloud_count < self.config.max_cloud_nodes {
            let needed = ((depth as usize - self.config.queue_depth_threshold as usize) / 5).max(1);
            let can_add = self.config.max_cloud_nodes - cloud_count;
            return ScaleDecision::ScaleUp(needed.min(can_add));
        }

        if !idle.is_empty() && cloud_count > self.config.min_cloud_nodes {
            let can_remove = cloud_count - self.config.min_cloud_nodes;
            let to_remove = idle.len().min(can_remove);
            if to_remove > 0 {
                return ScaleDecision::ScaleDown(to_remove);
            }
        }

        ScaleDecision::NoChange
    }

    pub async fn run_loop(self: Arc<Self>) {
        tracing::info!("autoscaler loop started");
        loop {
            let decision = self.evaluate().await;
            match decision {
                ScaleDecision::ScaleUp(n) => {
                    tracing::info!(count = n, "autoscaler: scale up requested");
                    // Integration point: trigger cloud provider API to launch GPU instances
                }
                ScaleDecision::ScaleDown(n) => {
                    tracing::info!(count = n, "autoscaler: scale down requested");
                    let idle = self.nodes.idle_nodes(self.config.idle_timeout_secs);
                    for node in idle.into_iter().take(n) {
                        self.nodes.set_status(node.id, omni_core::WorkerStatus::Draining);
                        tracing::info!(worker_id = %node.id, "marking node for removal");
                    }
                }
                ScaleDecision::NoChange => {}
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}
