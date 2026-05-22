use thiserror::Error;

#[derive(Debug, Error)]
pub enum OmniError {
    #[error("LLM request failed: {0}")]
    LlmError(String),
    #[error("Asset generation failed: {0}")]
    AssetError(String),
    #[error("Code generation failed: {0}")]
    CodegenError(String),
    #[error("Pipeline error: {0}")]
    PipelineError(String),
    #[error("Scheduler error: {0}")]
    SchedulerError(String),
    #[error("Worker error: {0}")]
    WorkerError(String),
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Quota exceeded for user {0}")]
    QuotaExceeded(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}
