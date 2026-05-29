use async_trait::async_trait;
use futures::Stream;

use crate::agent::event::EnvEvent;

#[async_trait]
pub trait EventContext<E: EnvEvent>: Send + Sync + 'static {
    fn push_event(&self, event: E);
    fn size(&self) -> usize;
    async fn chunk_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>;
}
