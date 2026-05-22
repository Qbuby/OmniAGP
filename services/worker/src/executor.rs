use anyhow::Result;
use omni_core::{TaskCapability, TaskPayload};

pub async fn execute_task(task: &TaskPayload) -> Result<serde_json::Value> {
    match task.capability {
        TaskCapability::Image2D => execute_image_2d(task).await,
        TaskCapability::Model3D => execute_model_3d(task).await,
        TaskCapability::Audio => execute_audio(task).await,
        TaskCapability::LlmInference => execute_llm(task).await,
        TaskCapability::General => execute_general(task).await,
    }
}

async fn execute_image_2d(task: &TaskPayload) -> Result<serde_json::Value> {
    tracing::info!(task_id = %task.id, "executing 2D image generation (SDXL)");
    let prompt = task.payload.get("prompt").and_then(|v| v.as_str()).unwrap_or("default");

    // Integration point: call SDXL inference via ComfyUI/A1111 API or direct model loading
    let result = serde_json::json!({
        "type": "image_2d",
        "prompt": prompt,
        "status": "generated",
        "output_path": format!("/outputs/{}/image.png", task.id),
    });

    Ok(result)
}

async fn execute_model_3d(task: &TaskPayload) -> Result<serde_json::Value> {
    tracing::info!(task_id = %task.id, "executing 3D model generation (TripoSR)");
    let prompt = task.payload.get("prompt").and_then(|v| v.as_str()).unwrap_or("default");

    // Integration point: call TripoSR/Hunyuan3D inference
    let result = serde_json::json!({
        "type": "model_3d",
        "prompt": prompt,
        "status": "generated",
        "output_path": format!("/outputs/{}/model.glb", task.id),
    });

    Ok(result)
}

async fn execute_audio(task: &TaskPayload) -> Result<serde_json::Value> {
    tracing::info!(task_id = %task.id, "executing audio generation (MusicGen)");
    let prompt = task.payload.get("prompt").and_then(|v| v.as_str()).unwrap_or("default");

    // Integration point: call MusicGen inference
    let result = serde_json::json!({
        "type": "audio",
        "prompt": prompt,
        "status": "generated",
        "output_path": format!("/outputs/{}/audio.wav", task.id),
    });

    Ok(result)
}

async fn execute_llm(task: &TaskPayload) -> Result<serde_json::Value> {
    tracing::info!(task_id = %task.id, "executing LLM inference");
    let prompt = task.payload.get("prompt").and_then(|v| v.as_str()).unwrap_or("");

    let base_url = std::env::var("LLM_BASE_URL").unwrap_or_default();
    let api_key = std::env::var("LLM_API_KEY").unwrap_or_default();
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5-coder-7b".into());

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", base_url))
        .bearer_auth(&api_key)
        .json(&serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(resp)
}

async fn execute_general(task: &TaskPayload) -> Result<serde_json::Value> {
    tracing::info!(task_id = %task.id, "executing general task");
    Ok(serde_json::json!({
        "type": "general",
        "status": "completed",
        "task_id": task.id.to_string(),
    }))
}
