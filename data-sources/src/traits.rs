use async_trait::async_trait;
use graph_core::NewContent;

#[async_trait]
pub trait DataSource: Send + Sync {
    async fn load(&self) -> anyhow::Result<Vec<NewContent>>;
    fn name(&self) -> &str;
}
