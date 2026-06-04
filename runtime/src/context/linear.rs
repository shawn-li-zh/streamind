use std::sync::{Arc, atomic::AtomicUsize};

use async_trait::async_trait;
use kernel::{
    client::ChatClient,
    message::RoleMessage,
    mind::{
        context::{ChunkStream, EventContext},
        event::EnvEvent,
    },
};
use parking_lot::RwLock;

pub struct LinearContext<E> {
    system_prompt: RoleMessage,
    client: Arc<dyn ChatClient>,
    events: RwLock<Vec<Arc<E>>>,
    //上下文长度
    content_size: AtomicUsize,
}
#[async_trait]
impl<E: EnvEvent> EventContext<E> for LinearContext<E> {
    fn push_event<T: Into<E>>(&self, event: T) {
        let event = event.into();
        let size = event.size();
        self.content_size
            .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
        let mut events = self.events.write();
        events.push(Arc::new(event));
    }

    fn size(&self) -> usize {
        self.content_size.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn chunk_stream(&self) -> anyhow::Result<ChunkStream> {
        let messages = self.messages();
        let llm_stream = self.client.stream(messages).await?;
        Ok(llm_stream)
    }
}

impl<E: EnvEvent> LinearContext<E> {
    pub fn messages(&self) -> Vec<RoleMessage> {
        let mut msgs = vec![self.system_prompt.clone()];
        let events = self.events.read();
        for event in &*events {
            msgs.push(event.to_message());
        }
        msgs
    }
}

impl<E> LinearContext<E> {
    pub fn new(system_prompt: String, client: Arc<dyn ChatClient>) -> Self {
        Self {
            system_prompt: RoleMessage::System(vec![system_prompt.into()]),
            client,
            events: Default::default(),
            content_size: Default::default(),
        }
    }
    pub fn clear(&self) {
        self.events.write().clear();
        self.content_size
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}
