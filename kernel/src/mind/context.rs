use async_trait::async_trait;
use futures::Stream;

use crate::mind::event::EnvEvent;
pub type ChunkStream = std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>;
#[async_trait]
pub trait EventContext<E: EnvEvent>: Send + Sync + 'static {
    fn push_event<T:Into<E>>(&self, event: T);
    fn size(&self) -> usize;
    async fn chunk_stream(&self) -> anyhow::Result<ChunkStream>;
}
