use crate::message::Information;

#[async_trait::async_trait]
pub trait EmbeddingClient: Send + Sync + 'static {
    async fn batch_embed(&self, infos: Vec<Information>) -> anyhow::Result<Vec<Vec<f32>>>;
}
