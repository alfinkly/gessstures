use async_trait::async_trait;
use graph_core::NodeData;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>>;
}

#[async_trait]
pub trait LlmReasoner: Send + Sync {
    async fn reason(
        &self,
        prompt: &str,
        context: &[NodeData],
    ) -> anyhow::Result<String>;
}

// http_backend module is declared in lib.rs
