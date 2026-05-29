use std::sync::{Arc, LazyLock, atomic::AtomicUsize};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use kernel::{
    agent::event::{self, EnvEvent},
    client::ChatClient,
    message::{Role, RoleMessage},
    mind::context::EventContext,
};

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// test
pub(crate) static OUTPUTS: LazyLock<Vec<Vec<String>>> = LazyLock::new(|| {
    if let Ok(test) = std::fs::read_to_string("./test.xml") {
        return test
            .split("<end/>")
            .map(|item| item.trim())
            .filter(|v| !v.is_empty())
            .map(|item| split_into_chunks(item))
            .collect();
    }
    vec![]
});

pub fn split_into_chunks(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect(); // 转为字符数组
    let mut chunks = Vec::new();
    let mut start = 0;
    let len = chars.len();

    while start < len {
        // 随机决定当前块的长度
        let chunk_size = 2;
        let end = (start + chunk_size).min(len);

        // 获取切片并转回 String
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        start = end;
    }

    chunks
}
// struct StackContextInner<E> {
//     pub nodes: Vec<Arc<MindNode<E>>>,
//     //上下文长度
//     pub content_len: usize,
// }

// impl<E> StackContextInner<E> {
//     pub fn new() -> Self {
//         Self {
//             nodes: vec![],

//             content_len: 0,
//         }
//     }

//     pub fn push(&mut self, node: Node) {
//         let node = MindNode {
//             node,
//             events: RwLock::new(vec![]),
//             content_len: 0,
//         };
//         self.back_to(node.node.pos);
//         self.nodes.push(node);
//     }

//     //回到指定位置，并丢弃后面的节点
//     fn back_to(&mut self, pos: usize) -> Vec<MindNode<E>> {
//         let removed = self.nodes.split_off(pos);
//         for ele in &removed {
//             self.content_len = self.content_len.saturating_sub(ele.content_len)
//         }
//         return removed;
//     }

//     pub fn last(&self) -> Option<&MindNode<E>> {
//         self.nodes.last()
//     }

//     // pub async fn chunk_stream(
//     //     &self,
//     //     client: Arc<dyn ChatClient>,
//     // ) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
//     //     let o = &*OUTPUTS;
//     //     let i = self.step % o.len();
//     //     stream::iter(o[i].clone()).boxed()
//     // }
// }

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
}
pub struct MindNode<E> {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: RwLock<String>,
    events: RwLock<Vec<Arc<E>>>,
    //上下文长度
    content_size: AtomicUsize,
}

impl<E> MindNode<E> {
    pub fn new(node: Node) -> Self {
        Self {
            id: node.id,
            pos: node.pos,
            title: node.title,
            content: RwLock::new(node.content),
            events: RwLock::new(vec![]),
            content_size: AtomicUsize::new(0),
        }
    }
    pub fn statement(&self) -> String {
        format!(
            r#"<stack pos={} title="{}">
            {}
            </stack>"#,
            self.pos,
            self.title,
            self.content.read()
        )
    }
    pub fn size(&self) -> usize {
        self.content_size.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn len(&self) -> usize {
        let events = self.events.read();
        events.len()
    }

    pub fn events(&self) -> Vec<Arc<E>> {
        let events = self.events.read();
        events.clone()
    }

    pub fn reset(&self, content: String) {
        let mut c = self.content.write();
        *c = content;
        self.events.write().clear();
    }
}
impl<E: EnvEvent> MindNode<E> {
    pub fn push(&self, event: E) {
        let mut events = self.events.write();
        self.content_size
            .fetch_add(event.size(), std::sync::atomic::Ordering::Relaxed);
        events.push(Arc::new(event));
    }
    pub fn to_messages(&self) -> Vec<RoleMessage> {
        let events = self.events.read();
        let mut msgs = Vec::with_capacity(events.len() + 1);
        //里面的内容都是第一人称
        let state = RoleMessage::Assistant(vec![self.statement().into()], false);
        msgs.push(state);
        let messages: Vec<RoleMessage> = events.iter().map(|e| e.to_message()).collect();
        msgs.extend(messages);
        msgs
    }
}

pub struct StackContext<E> {
    pub system_prompt: RoleMessage,
    pub client: Arc<dyn ChatClient>,
    pub nodes: RwLock<Vec<Arc<MindNode<E>>>>,
    //上下文长度
    pub content_size: AtomicUsize,
}
impl<E> StackContext<E> {
    pub fn new(client: Arc<dyn ChatClient>, system_prompt: String) -> Self {
        Self {
            client,
            system_prompt: RoleMessage::System(vec![system_prompt.into()]),
            nodes: RwLock::new(vec![]),
            content_size: AtomicUsize::new(0),
        }
    }
    pub fn nodes(&self) -> Vec<Arc<MindNode<E>>> {
        let inner = self.nodes.read();
        inner.clone()
    }
    pub fn top_node(&self) -> Option<Arc<MindNode<E>>> {
        let inner = self.nodes.read();
        inner.last().cloned()
    }
    pub fn push_node(&self, node: Node) -> Arc<MindNode<E>> {
        let node = MindNode::new(node);
        let mut nodes = self.nodes.write();
        let arc_node = Arc::new(node);
        nodes.push(arc_node.clone());
        arc_node
    }
    pub fn pop_node(&self) -> Option<Arc<MindNode<E>>> {
        let mut nodes = self.nodes.write();
        nodes.pop()
    }

    //     //回到指定位置，并丢弃后面的节点
    fn back_to(&self, pos: usize) -> Vec<Arc<MindNode<E>>> {
        let mut nodes = self.nodes.write();
        let removed = nodes.split_off(pos);
        for ele in &removed {
            let content_size = self.content_size.load(std::sync::atomic::Ordering::Relaxed);
            self.content_size.store(
                content_size.saturating_sub(ele.size()),
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        return removed;
    }
    pub fn node_len(&self) -> usize {
        return self.nodes.read().len();
    }
}

#[async_trait]
impl<E: EnvEvent> EventContext<E> for StackContext<E> {
    fn push_event(&self, event: E) {
        let size = event.size();
        self.content_size
            .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
        let inner = self.nodes.read();
        let node: &MindNode<E> = inner.last().unwrap();
        node.push(event);
    }

    fn size(&self) -> usize {
        self.content_size.load(std::sync::atomic::Ordering::Relaxed)
    }
    async fn chunk_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>> {
        let messages = self.messages();
        // println!("{:#?}", &messages);
        let llm_stream = self
            .client
            .stream(messages)
            .await
            .expect("Failed to llm stream");

        llm_stream
    }
}
impl<E: EnvEvent> StackContext<E> {
    fn messages(&self) -> Vec<RoleMessage> {
        let mut msgs = vec![self.system_prompt.clone()];
        let inner = self.nodes.read();
        for node in &*inner {
            msgs.extend(node.to_messages());
        }
        msgs
    }
}

pub enum StackCommand {
    Push(Node),
    Reset(String),
    Pop(String),
}
