#[derive(Debug, Clone)]
pub enum CommandEvent {
    Generate,
    Canceled,
}

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{SinkExt as _, Stream};
use pin_project::pin_project;
use tokio::sync::mpsc::{self, error::SendError};
use tokio_stream::wrappers::ReceiverStream;

use crate::section::SectionTag;
pub struct MindInlet {
    inner: mpsc::Sender<CommandEvent>,
}
impl MindInlet {
    pub(crate) fn new(inner: mpsc::Sender<CommandEvent>) -> Self {
        Self { inner: inner }
    }
    pub async fn send(&self, event: CommandEvent) -> Result<(), SendError<CommandEvent>> {
        self.inner.send(event).await
    }
}

#[derive(Debug, Clone)]
pub enum GenerateEvent {
    Start(Vec<i64>),
    Open(&'static SectionTag, String),
    Chunk(&'static SectionTag, String),
    Close(&'static SectionTag, String),
    End(Vec<i64>, EndReason),
    Error(Arc<anyhow::Error>),
}
#[derive(Debug, Clone)]
pub enum EndReason {
    Normal,
    Error,
    Canceled,
}
#[pin_project] // 1. 在结构体上标注
pub struct MindOutlet {
    #[pin] // 2. 标注需要被投影的字段
    inner: ReceiverStream<GenerateEvent>,
}

impl MindOutlet {
    pub(crate) fn new(rx: tokio::sync::mpsc::Receiver<GenerateEvent>) -> Self {
        Self {
            inner: ReceiverStream::new(rx),
        }
    }
}

impl Stream for MindOutlet {
    type Item = GenerateEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}
