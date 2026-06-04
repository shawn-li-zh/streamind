// use std::{
//     pin::Pin,
//     task::{Context, Poll},
// };

// use futures::Stream;
// use pin_project::pin_project;
// use tokio_stream::wrappers::ReceiverStream;



// #[pin_project] // 1. 在结构体上标注
// pub struct AgentOutlet<E> {
//     #[pin] // 2. 标注需要被投影的字段
//     inner: ReceiverStream<AgentEvent<E>>,
// }

// impl<E> AgentOutlet<E> {
//     pub fn new(rx: tokio::sync::mpsc::Receiver<AgentEvent<E>>) -> Self {
//         Self {
//             inner: ReceiverStream::new(rx),
//         }
//     }
// }

// impl<E> Stream for AgentOutlet<E> {
//     type Item = AgentEvent<E>;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let this = self.project();
//         this.inner.poll_next(cx)
//     }
// }
