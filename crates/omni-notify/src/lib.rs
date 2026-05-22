use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    GenerationComplete,
    GenerationFailed,
    QaFailed,
    ReviewRequested,
    ReviewApproved,
    ReviewRejected,
    GamePublished,
    MemberInvited,
    WorkflowTransition,
    GddEdited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_type: EventType,
    pub title: String,
    pub message: String,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl NotificationEvent {
    pub fn new(
        project_id: Uuid,
        event_type: EventType,
        title: String,
        message: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            event_type,
            title,
            message,
            metadata: serde_json::Value::Null,
            timestamp: Utc::now(),
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: Uuid,
    pub project_id: Uuid,
    pub url: String,
    pub events: Vec<EventType>,
    pub adapter_type: AdapterType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdapterType {
    Slack,
    Discord,
    Feishu,
    Email,
    Generic,
}

#[async_trait]
pub trait NotificationAdapter: Send + Sync {
    async fn send(&self, event: &NotificationEvent, config: &WebhookConfig) -> anyhow::Result<()>;
    fn adapter_type(&self) -> AdapterType;
}

pub struct SlackAdapter {
    client: reqwest::Client,
}

impl SlackAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for SlackAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationAdapter for SlackAdapter {
    async fn send(&self, event: &NotificationEvent, config: &WebhookConfig) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "text": format!("*{}*\n{}", event.title, event.message),
            "blocks": [{
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("*{}*\n{}", event.title, event.message)
                }
            }]
        });

        self.client
            .post(&config.url)
            .json(&payload)
            .send()
            .await?;
        Ok(())
    }

    fn adapter_type(&self) -> AdapterType {
        AdapterType::Slack
    }
}

pub struct DiscordAdapter {
    client: reqwest::Client,
}

impl DiscordAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DiscordAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationAdapter for DiscordAdapter {
    async fn send(&self, event: &NotificationEvent, config: &WebhookConfig) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "embeds": [{
                "title": event.title,
                "description": event.message,
                "color": match event.event_type {
                    EventType::GenerationComplete | EventType::ReviewApproved | EventType::GamePublished => 0x00ff00u32,
                    EventType::GenerationFailed | EventType::QaFailed | EventType::ReviewRejected => 0xff0000u32,
                    _ => 0x0099ffu32,
                },
                "timestamp": event.timestamp.to_rfc3339(),
            }]
        });

        self.client
            .post(&config.url)
            .json(&payload)
            .send()
            .await?;
        Ok(())
    }

    fn adapter_type(&self) -> AdapterType {
        AdapterType::Discord
    }
}

pub struct GenericWebhookAdapter {
    client: reqwest::Client,
}

impl GenericWebhookAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for GenericWebhookAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationAdapter for GenericWebhookAdapter {
    async fn send(&self, event: &NotificationEvent, config: &WebhookConfig) -> anyhow::Result<()> {
        self.client
            .post(&config.url)
            .json(event)
            .send()
            .await?;
        Ok(())
    }

    fn adapter_type(&self) -> AdapterType {
        AdapterType::Generic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

pub struct EventBus {
    adapters: HashMap<String, Arc<dyn NotificationAdapter>>,
    webhooks: Vec<WebhookConfig>,
    feed: Arc<RwLock<Vec<FeedEntry>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let mut adapters: HashMap<String, Arc<dyn NotificationAdapter>> = HashMap::new();
        adapters.insert("slack".into(), Arc::new(SlackAdapter::new()));
        adapters.insert("discord".into(), Arc::new(DiscordAdapter::new()));
        adapters.insert("generic".into(), Arc::new(GenericWebhookAdapter::new()));

        Self {
            adapters,
            webhooks: Vec::new(),
            feed: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn register_webhook(&mut self, config: WebhookConfig) {
        self.webhooks.push(config);
    }

    pub fn remove_webhook(&mut self, webhook_id: Uuid) {
        self.webhooks.retain(|w| w.id != webhook_id);
    }

    pub async fn emit(&self, event: NotificationEvent) {
        let feed_entry = FeedEntry {
            id: event.id,
            project_id: event.project_id,
            title: event.title.clone(),
            content: event.message.clone(),
            timestamp: event.timestamp,
        };
        self.feed.write().await.push(feed_entry);

        for webhook in &self.webhooks {
            if !webhook.enabled {
                continue;
            }
            if webhook.project_id != event.project_id {
                continue;
            }

            let matches_event = webhook.events.iter().any(|e| {
                std::mem::discriminant(e) == std::mem::discriminant(&event.event_type)
            });
            if !matches_event {
                continue;
            }

            let adapter_key = match webhook.adapter_type {
                AdapterType::Slack => "slack",
                AdapterType::Discord => "discord",
                AdapterType::Feishu | AdapterType::Email | AdapterType::Generic => "generic",
            };

            if let Some(adapter) = self.adapters.get(adapter_key) {
                let adapter = adapter.clone();
                let event = event.clone();
                let config = webhook.clone();
                tokio::spawn(async move {
                    if let Err(e) = adapter.send(&event, &config).await {
                        tracing::error!(error = %e, "failed to send notification");
                    }
                });
            }
        }
    }

    pub async fn get_feed(&self, project_id: Uuid, limit: usize) -> Vec<FeedEntry> {
        let feed = self.feed.read().await;
        feed.iter()
            .rev()
            .filter(|e| e.project_id == project_id)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn generate_atom_feed(&self, entries: &[FeedEntry], project_name: &str) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        xml.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
        xml.push_str(&format!("  <title>{} Activity Feed</title>\n", project_name));
        xml.push_str(&format!("  <id>urn:uuid:{}</id>\n", Uuid::new_v4()));
        if let Some(latest) = entries.first() {
            xml.push_str(&format!("  <updated>{}</updated>\n", latest.timestamp.to_rfc3339()));
        }

        for entry in entries {
            xml.push_str("  <entry>\n");
            xml.push_str(&format!("    <title>{}</title>\n", entry.title));
            xml.push_str(&format!("    <id>urn:uuid:{}</id>\n", entry.id));
            xml.push_str(&format!("    <updated>{}</updated>\n", entry.timestamp.to_rfc3339()));
            xml.push_str(&format!("    <content type=\"text\">{}</content>\n", entry.content));
            xml.push_str("  </entry>\n");
        }

        xml.push_str("</feed>\n");
        xml
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
