use crate::message::Information;

#[async_trait::async_trait]
pub trait EmbeddingClient: Send + Sync + 'static {
    //多模态信息联合嵌入
    async fn union_embed(&self, infos: Vec<Information>) -> anyhow::Result<Vec<f32>>;
}
