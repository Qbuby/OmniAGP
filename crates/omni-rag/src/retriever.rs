use anyhow::Result;
use tracing::info;

use crate::embedding::EmbeddingClient;
use crate::qdrant::{QdrantClient, SearchResult};
use crate::templates::TemplateLibrary;

const COLLECTION_NAME: &str = "godot_knowledge";
const TOP_K: usize = 8;

pub struct RagRetriever {
    qdrant: QdrantClient,
    embedder: EmbeddingClient,
    templates: TemplateLibrary,
}

#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub snippets: Vec<RetrievedSnippet>,
    pub matched_template: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RetrievedSnippet {
    pub content: String,
    pub source: String,
    pub score: f32,
}

impl RagRetriever {
    pub fn new(qdrant: QdrantClient, embedder: EmbeddingClient, templates: TemplateLibrary) -> Self {
        Self {
            qdrant,
            embedder,
            templates,
        }
    }

    pub fn from_env() -> Result<Self> {
        let qdrant = QdrantClient::from_env();
        let embedder = EmbeddingClient::from_env()?;
        let templates = TemplateLibrary::load_builtin();
        Ok(Self::new(qdrant, embedder, templates))
    }

    pub async fn retrieve(&self, task_description: &str) -> Result<RetrievalResult> {
        let matched_template = self.templates.find_match(task_description);
        if let Some(ref tmpl) = matched_template {
            info!(template = %tmpl.split('\n').next().unwrap_or(""), "template match found");
        }

        let query_vec = self.embedder.embed_single(task_description).await?;
        let results = self.qdrant.search(COLLECTION_NAME, query_vec, TOP_K).await?;

        let snippets: Vec<RetrievedSnippet> = results
            .into_iter()
            .filter_map(|r| {
                let payload = r.payload?;
                Some(RetrievedSnippet {
                    content: payload["content"].as_str().unwrap_or("").to_string(),
                    source: payload["source"].as_str().unwrap_or("unknown").to_string(),
                    score: r.score,
                })
            })
            .collect();

        info!(
            snippets = snippets.len(),
            has_template = matched_template.is_some(),
            "retrieval complete"
        );

        Ok(RetrievalResult {
            snippets,
            matched_template,
        })
    }

    pub fn build_context_prompt(&self, result: &RetrievalResult) -> String {
        let mut context = String::new();

        if let Some(ref template) = result.matched_template {
            context.push_str("## Matching Template (use as base, customize as needed):\n```gdscript\n");
            context.push_str(template);
            context.push_str("\n```\n\n");
        }

        if !result.snippets.is_empty() {
            context.push_str("## Relevant Godot 4 API References:\n");
            for (i, snippet) in result.snippets.iter().enumerate() {
                context.push_str(&format!(
                    "\n### Reference {} (source: {}, relevance: {:.2}):\n{}\n",
                    i + 1,
                    snippet.source,
                    snippet.score,
                    snippet.content
                ));
            }
        }

        context
    }

    pub async fn ensure_collection(&self, vector_size: usize) -> Result<()> {
        if !self.qdrant.collection_exists(COLLECTION_NAME).await? {
            self.qdrant
                .create_collection(COLLECTION_NAME, vector_size)
                .await?;
        }
        Ok(())
    }

    pub fn qdrant(&self) -> &QdrantClient {
        &self.qdrant
    }

    pub fn embedder(&self) -> &EmbeddingClient {
        &self.embedder
    }
}
