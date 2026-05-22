use anyhow::Result;
use prometheus::{IntCounter, IntGauge, Histogram, Registry, opts, histogram_opts};

pub struct SchedulerMetrics {
    pub registry: Registry,
    pub tasks_submitted: IntCounter,
    pub tasks_dispatched: IntCounter,
    pub tasks_completed: IntCounter,
    pub tasks_failed: IntCounter,
    pub queue_depth: IntGauge,
    pub online_workers: IntGauge,
    pub dispatch_latency: Histogram,
}

impl SchedulerMetrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        let tasks_submitted = IntCounter::with_opts(opts!("scheduler_tasks_submitted_total", "Total tasks submitted"))?;
        let tasks_dispatched = IntCounter::with_opts(opts!("scheduler_tasks_dispatched_total", "Total tasks dispatched"))?;
        let tasks_completed = IntCounter::with_opts(opts!("scheduler_tasks_completed_total", "Total tasks completed"))?;
        let tasks_failed = IntCounter::with_opts(opts!("scheduler_tasks_failed_total", "Total tasks failed"))?;
        let queue_depth = IntGauge::with_opts(opts!("scheduler_queue_depth", "Current queue depth"))?;
        let online_workers = IntGauge::with_opts(opts!("scheduler_online_workers", "Number of online workers"))?;
        let dispatch_latency = Histogram::with_opts(histogram_opts!(
            "scheduler_dispatch_latency_seconds",
            "Task dispatch latency",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0]
        ))?;

        registry.register(Box::new(tasks_submitted.clone()))?;
        registry.register(Box::new(tasks_dispatched.clone()))?;
        registry.register(Box::new(tasks_completed.clone()))?;
        registry.register(Box::new(tasks_failed.clone()))?;
        registry.register(Box::new(queue_depth.clone()))?;
        registry.register(Box::new(online_workers.clone()))?;
        registry.register(Box::new(dispatch_latency.clone()))?;

        Ok(Self {
            registry,
            tasks_submitted,
            tasks_dispatched,
            tasks_completed,
            tasks_failed,
            queue_depth,
            online_workers,
            dispatch_latency,
        })
    }
}
