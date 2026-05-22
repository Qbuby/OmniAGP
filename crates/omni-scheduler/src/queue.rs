use anyhow::Result;
use omni_core::{TaskPayload, TaskPriority, TaskResult};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct TaskQueue {
    client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    pending: Arc<Mutex<Vec<TaskPayload>>>,
}

const STREAM_NAME: &str = "OMNI_TASKS";
const SUBJECT_PREFIX: &str = "tasks";

impl TaskQueue {
    pub async fn connect(nats_url: &str) -> Result<Self> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = async_nats::jetstream::new(client.clone());

        let stream_config = async_nats::jetstream::stream::Config {
            name: STREAM_NAME.to_string(),
            subjects: vec![format!("{}.>", SUBJECT_PREFIX)],
            retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
            ..Default::default()
        };

        jetstream.get_or_create_stream(stream_config).await?;

        Ok(Self {
            client,
            jetstream,
            pending: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn publish(&self, task: TaskPayload) -> Result<()> {
        let subject = format!("{}.{}", SUBJECT_PREFIX, priority_subject(&task.priority));
        let payload = serde_json::to_vec(&task)?;
        self.jetstream.publish(subject, payload.into()).await?.await?;
        tracing::debug!(task_id = %task.id, "task published to queue");
        Ok(())
    }

    pub async fn pull_next(&self) -> Result<Option<TaskPayload>> {
        let mut pending = self.pending.lock().await;
        if let Some(task) = pending.pop() {
            return Ok(Some(task));
        }
        drop(pending);

        let stream = self.jetstream.get_stream(STREAM_NAME).await?;
        let consumer_config = async_nats::jetstream::consumer::pull::Config {
            durable_name: Some("scheduler".to_string()),
            filter_subject: format!("{}.>", SUBJECT_PREFIX),
            ..Default::default()
        };
        let consumer = stream.get_or_create_consumer("scheduler", consumer_config).await?;

        let mut messages = consumer.fetch().max_messages(1).messages().await?;
        use futures::StreamExt;
        if let Some(Ok(msg)) = messages.next().await {
            let task: TaskPayload = serde_json::from_slice(&msg.payload)?;
            msg.ack().await.map_err(|e| anyhow::anyhow!("{}", e))?;
            return Ok(Some(task));
        }

        Ok(None)
    }

    pub async fn nack(&self, task: TaskPayload) -> Result<()> {
        let mut pending = self.pending.lock().await;
        pending.push(task);
        Ok(())
    }

    pub async fn send_to_worker(&self, worker_id: Uuid, task: TaskPayload) -> Result<()> {
        let subject = format!("worker.{}.tasks", worker_id);
        let payload = serde_json::to_vec(&task)?;
        self.client.publish(subject, payload.into()).await?;
        Ok(())
    }

    pub async fn ack_result(&self, result: TaskResult) -> Result<()> {
        let subject = "tasks.results";
        let payload = serde_json::to_vec(&result)?;
        self.client.publish(subject.to_string(), payload.into()).await?;
        tracing::debug!(task_id = %result.task_id, "result acknowledged");
        Ok(())
    }

    pub async fn dead_letter(&self, task: TaskPayload, error: &str) -> Result<()> {
        let subject = format!("{}.deadletter", SUBJECT_PREFIX);
        let dlq_entry = serde_json::json!({
            "task": task,
            "error": error,
            "timestamp": chrono::Utc::now().timestamp(),
        });
        let payload = serde_json::to_vec(&dlq_entry)?;
        self.jetstream.publish(subject, payload.into()).await?.await?;
        tracing::warn!(task_id = %task.id, error, "task moved to dead letter queue");
        Ok(())
    }

    pub async fn queue_depth(&self) -> Result<u64> {
        let mut stream = self.jetstream.get_stream(STREAM_NAME).await?;
        let info = stream.info().await?;
        Ok(info.state.messages)
    }
}

fn priority_subject(priority: &TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Urgent => "urgent",
        TaskPriority::Normal => "normal",
        TaskPriority::Batch => "batch",
    }
}
