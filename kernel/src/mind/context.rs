use async_trait::async_trait;
use futures::Stream;

use crate::agent::event::EnvEvent;

#[async_trait]
pub trait EventContext<E: EnvEvent>: Send + Sync + 'static {
    fn push_events(&self, events: Vec<E>);
    async fn chunk_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>;
}
