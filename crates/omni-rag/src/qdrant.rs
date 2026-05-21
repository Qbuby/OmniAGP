use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantPoint {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct UpsertRequest {
    points: Vec<UpsertPoint>,
}

#[derive(Debug, Clone, Serialize)]
struct UpsertPoint {
    id: String,
    vector: Vec<f32>,
    payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct SearchRequest {
    vector: Vec<f32>,
    limit: usize,
    with_payload: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    result: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub id: serde_json::Value,
    pub score: f32,
    pub payload: Option<serde_json::Value>,
}

pub struct QdrantClient {
    http: Client,
    base_url: String,
}

impl QdrantClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn from_env() -> Self {
        let base_url =
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
        Self::new(base_url)
    }

    pub async fn create_collection(
        &self,
        name: &str,
        vector_size: usize,
    ) -> Result<()> {
        let url = format!("{}/collections/{}", self.base_url, name);
        let body = serde_json::json!({
            "vectors": {
                "size": vector_size,
                "distance": "Cosine"
            }
        });

        let resp = self.http.put(&url).json(&body).send().await?;
        if resp.status().is_success() || resp.status().as_u16() == 409 {
            info!(collection = name, "collection ready");
            Ok(())
        } else {
            let text = resp.text().await?;
            anyhow::bail!("failed to create collection {}: {}", name, text)
        }
    }

    pub async fn upsert(
        &self,
        collection: &str,
        points: Vec<QdrantPoint>,
    ) -> Result<()> {
        let url = format!("{}/collections/{}/points", self.base_url, collection);
        let upsert_points: Vec<UpsertPoint> = points
            .into_iter()
            .map(|p| UpsertPoint {
                id: p.id,
                vector: p.vector,
                payload: p.payload,
            })
            .collect();

        let body = UpsertRequest {
            points: upsert_points,
        };

        self.http
            .put(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        info!(collection = collection, "points upserted");
        Ok(())
    }

    pub async fn search(
        &self,
        collection: &str,
        vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let url = format!("{}/collections/{}/points/search", self.base_url, collection);
        let body = SearchRequest {
            vector,
            limit,
            with_payload: true,
        };

        let response: SearchResponse = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        info!(collection = collection, results = response.result.len(), "search complete");
        Ok(response.result)
    }

    pub async fn collection_exists(&self, name: &str) -> Result<bool> {
        let url = format!("{}/collections/{}", self.base_url, name);
        let resp = self.http.get(&url).send().await?;
        Ok(resp.status().is_success())
    }
}
