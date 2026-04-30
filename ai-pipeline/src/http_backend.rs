use async_trait::async_trait;
use graph_core::NodeData;
use serde::{Deserialize, Serialize};

use crate::traits::{EmbeddingProvider, LlmReasoner};

pub struct HttpAiBackend {
    pub base_url: String,
    client: reqwest::Client,
}

impl HttpAiBackend {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct EmbedRequest {
    texts: Vec<String>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[async_trait]
impl EmbeddingProvider for HttpAiBackend {
    async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        let resp = self
            .client
            .post(format!("{}/embed", self.base_url))
            .json(&EmbedRequest {
                texts: texts.to_vec(),
            })
            .send()
            .await?
            .json::<EmbedResponse>()
            .await?;
        Ok(resp.embeddings)
    }
}

#[derive(Serialize)]
struct ReasonRequest {
    prompt: String,
    context: Vec<NodeData>,
}

#[derive(Deserialize)]
struct ReasonResponse {
    result: String,
}

#[async_trait]
impl LlmReasoner for HttpAiBackend {
    async fn reason(
        &self,
        prompt: &str,
        context: &[NodeData],
    ) -> anyhow::Result<String> {
        let resp = self
            .client
            .post(format!("{}/reason", self.base_url))
            .json(&ReasonRequest {
                prompt: prompt.to_string(),
                context: context.to_vec(),
            })
            .send()
            .await?
            .json::<ReasonResponse>()
            .await?;
        Ok(resp.result)
    }
}
