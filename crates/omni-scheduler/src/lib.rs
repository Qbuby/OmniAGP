pub mod queue;
pub mod node_manager;
pub mod strategy;
pub mod autoscaler;
pub mod billing;
pub mod metrics;

use anyhow::Result;
use omni_core::{TaskPayload, TaskResult};
use std::sync::Arc;
use uuid::Uuid;

pub use queue::TaskQueue;
pub use node_manager::NodeManager;
pub use strategy::SchedulingStrategy;
pub use autoscaler::Autoscaler;
pub use billing::BillingManager;
pub use metrics::SchedulerMetrics;

pub struct Scheduler {
    pub queue: Arc<TaskQueue>,
    pub nodes: Arc<NodeManager>,
    pub strategy: SchedulingStrategy,
    pub autoscaler: Autoscaler,
    pub billing: Arc<BillingManager>,
    pub metrics: Arc<SchedulerMetrics>,
}

impl Scheduler {
    pub async fn new(nats_url: &str) -> Result<Self> {
        let queue = Arc::new(TaskQueue::connect(nats_url).await?);
        let nodes = Arc::new(NodeManager::new());
        let strategy = SchedulingStrategy::new();
        let metrics = Arc::new(SchedulerMetrics::new()?);
        let autoscaler = Autoscaler::new(queue.clone(), nodes.clone());
        let billing = Arc::new(BillingManager::new());

        Ok(Self { queue, nodes, strategy, autoscaler, billing, metrics })
    }

    pub async fn submit_task(&self, task: TaskPayload) -> Result<Uuid> {
        self.metrics.tasks_submitted.inc();
        let task_id = task.id;
        self.queue.publish(task).await?;
        Ok(task_id)
    }

    pub async fn dispatch_next(&self) -> Result<Option<(TaskPayload, Uuid)>> {
        let task = match self.queue.pull_next().await? {
            Some(t) => t,
            None => return Ok(None),
        };

        let online_nodes = self.nodes.online_nodes();
        let selected = self.strategy.select_node(&task, &online_nodes);

        match selected {
            Some(worker_id) => {
                self.metrics.tasks_dispatched.inc();
                self.nodes.increment_active_tasks(worker_id);
                Ok(Some((task, worker_id)))
            }
            None => {
                self.queue.nack(task).await?;
                Ok(None)
            }
        }
    }

    pub async fn report_result(&self, result: TaskResult) -> Result<()> {
        self.nodes.decrement_active_tasks(result.worker_id);

        match result.status {
            omni_core::TaskStatus::Completed => {
                self.metrics.tasks_completed.inc();
                self.billing.record_usage(result.gpu_minutes).await;
            }
            omni_core::TaskStatus::Failed => {
                self.metrics.tasks_failed.inc();
            }
            _ => {}
        }

        self.queue.ack_result(result).await?;
        Ok(())
    }

    pub fn autoscaler_handle(&self) -> Autoscaler {
        Autoscaler::new(self.queue.clone(), self.nodes.clone())
    }

    pub async fn run_dispatch_loop(self: Arc<Self>) {
        tracing::info!("scheduler dispatch loop started");
        loop {
            match self.dispatch_next().await {
                Ok(Some((task, worker_id))) => {
                    tracing::info!(task_id = %task.id, worker_id = %worker_id, "task dispatched");
                    if let Err(e) = self.queue.send_to_worker(worker_id, task).await {
                        tracing::error!(error = %e, "failed to send task to worker");
                    }
                }
                Ok(None) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "dispatch error");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}
