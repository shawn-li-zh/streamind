use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{SinkExt as _, Stream};
use pin_project::pin_project;
use tokio::sync::mpsc::{self, error::SendError};
use tokio_stream::wrappers::ReceiverStream;

use crate::mind::stream::GenerateEvent;
pub mod context;
pub mod error;
pub mod stream;

pub struct MindInlet<E> {
    inner: mpsc::Sender<E>,
}
impl<E> MindInlet<E> {
    pub async fn send(&self, event: E) -> Result<(), SendError<E>> {
        self.inner.send(event).await
    }
}

#[pin_project] // 1. 在结构体上标注
pub struct MindOutlet {
    #[pin] // 2. 标注需要被投影的字段
    inner: ReceiverStream<GenerateEvent>,
}

impl MindOutlet {
    pub fn new(rx: tokio::sync::mpsc::Receiver<GenerateEvent>) -> Self {
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
