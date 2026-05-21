use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    ArtifactCreated,
    BuildFailed,
    BuildSucceeded,
    FixRequested,
    TaskCompleted,
    TaskFailed,
    StateTransition,
    AgentMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: EventType,
    pub source: String,
    pub target: Option<String>,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl Event {
    pub fn new(event_type: EventType, source: &str, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            source: source.to_string(),
            target: None,
            payload,
            timestamp: Utc::now(),
        }
    }

    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }
}

#[derive(Clone)]
pub struct EventBus {
    sender: Arc<broadcast::Sender<Event>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
        }
    }

    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    pub fn publish_artifact_created(&self, source: &str, artifact_path: &str, artifact_type: &str) {
        self.publish(Event::new(
            EventType::ArtifactCreated,
            source,
            serde_json::json!({
                "path": artifact_path,
                "type": artifact_type,
            }),
        ));
    }

    pub fn publish_build_failed(&self, source: &str, error: &str, task_id: Uuid) {
        self.publish(Event::new(
            EventType::BuildFailed,
            source,
            serde_json::json!({
                "error": error,
                "task_id": task_id.to_string(),
            }),
        ));
    }

    pub fn publish_build_succeeded(&self, source: &str, output_path: &str) {
        self.publish(Event::new(
            EventType::BuildSucceeded,
            source,
            serde_json::json!({
                "output_path": output_path,
            }),
        ));
    }

    pub fn publish_fix_requested(&self, source: &str, task_id: Uuid, context: serde_json::Value) {
        self.publish(
            Event::new(
                EventType::FixRequested,
                source,
                serde_json::json!({
                    "task_id": task_id.to_string(),
                    "context": context,
                }),
            )
        );
    }

    pub fn publish_task_completed(&self, source: &str, task_id: Uuid, result: serde_json::Value) {
        self.publish(Event::new(
            EventType::TaskCompleted,
            source,
            serde_json::json!({
                "task_id": task_id.to_string(),
                "result": result,
            }),
        ));
    }

    pub fn publish_task_failed(&self, source: &str, task_id: Uuid, error: &str) {
        self.publish(Event::new(
            EventType::TaskFailed,
            source,
            serde_json::json!({
                "task_id": task_id.to_string(),
                "error": error,
            }),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pub_sub() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish_artifact_created("test_agent", "/output/sprite.png", "2d_sprite");

        let event = rx.recv().await.unwrap();
        assert!(matches!(event.event_type, EventType::ArtifactCreated));
        assert_eq!(event.source, "test_agent");
        assert_eq!(event.payload["path"], "/output/sprite.png");
    }
}
