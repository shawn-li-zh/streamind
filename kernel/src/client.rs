use async_trait::async_trait;

use crate::{message::RoleMessage, model::ModelFamily};

pub type ChunkStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, anyhow::Error>> + Send>>;

#[async_trait]
pub trait ChatClient: Send + Sync + 'static {
    fn model_info(&self) -> (ModelFamily, String);
    async fn stream(&self, messages: Vec<RoleMessage>) -> anyhow::Result<ChunkStream>;
}
