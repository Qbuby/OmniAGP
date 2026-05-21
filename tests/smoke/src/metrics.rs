use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct PerfMetrics {
    pub total_duration_ms: u64,
    pub pipeline_duration_ms: u64,
    pub llm_tokens_used: u64,
    pub asset_generation_ms: u64,
    pub code_generation_ms: u64,
    pub godot_assembly_ms: u64,
    pub qa_duration_ms: u64,
    pub export_duration_ms: u64,
}
