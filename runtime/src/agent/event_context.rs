use std::{
    fs::File,
    io::Write,
    sync::{Arc, LazyLock},
};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use kernel::{
    agent::event::EnvEvent,
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
struct StackContextInner<E> {
    pub nodes: Vec<MindNode<E>>,
    //上下文长度
    pub content_len: usize,
}
impl<E> StackContextInner<E> {
    pub fn new() -> Self {
        Self {
            nodes: vec![],

            content_len: 0,
        }
    }

    pub fn push(&mut self, node: Node) {
        let node = MindNode {
            node,
            events: vec![],
            content_len: 0,
        };
        self.back_to(node.node.pos);
        self.nodes.push(node);
    }

    //回到指定位置，并丢弃后面的节点
    fn back_to(&mut self, pos: usize) -> Vec<MindNode<E>> {
        let removed = self.nodes.split_off(pos);
        for ele in &removed {
            self.content_len -= ele.content_len;
        }
        return removed;
    }

    pub fn last(&self) -> Option<&MindNode<E>> {
        self.nodes.last()
    }
    fn last_mut(&mut self) -> Option<&mut MindNode<E>> {
        self.nodes.last_mut()
    }
    // pub async fn chunk_stream(
    //     &self,
    //     client: Arc<dyn ChatClient>,
    // ) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
    //     let o = &*OUTPUTS;
    //     let i = self.step % o.len();
    //     stream::iter(o[i].clone()).boxed()
    // }
}

pub struct StackContext<E> {
    pub system_prompt: RoleMessage,
    pub log_file: Mutex<File>,
    pub client: Arc<dyn ChatClient>,
    inner: RwLock<StackContextInner<E>>,
}
impl<E> StackContext<E> {
    pub fn new(client: Arc<dyn ChatClient>, system_prompt: String, log_file: File) -> Self {
        Self {
            client,
            log_file: Mutex::new(log_file),
            system_prompt: RoleMessage::System(vec![system_prompt.into()]),
            inner: RwLock::new(StackContextInner::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
}
pub struct MindNode<E> {
    pub node: Node,
    pub events: Vec<E>,
    //上下文长度
    pub content_len: usize,
}
impl<E: EnvEvent> MindNode<E> {
    pub fn statement(&self) -> String {
        format!(
            "<push pos={}>{}\n\n{}\n</push>",
            self.node.pos, self.node.title, self.node.content
        )
    }
    pub fn to_messages(&self) -> Vec<RoleMessage> {
        let mut msgs = Vec::with_capacity(self.events.len() + 1);
        //里面的内容都是第一人称
        let state = RoleMessage::Assistant(vec![self.statement().into()], false);
        msgs.push(state);
        let messages: Vec<RoleMessage> = self.events.iter().map(|e| e.to_message()).collect();
        msgs.extend(messages);
        msgs
    }

    pub fn push(&mut self, event: E) {
        // self.content_len += msg.size();
        self.events.push(event);
    }
}
#[async_trait]
impl<E: EnvEvent> EventContext<E> for StackContext<E> {
    fn push_events(&self, events: Vec<E>) {
        info!("received events [{}]", events.len());
        for event in events {
            self.push_event(event);
        }
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
        let inner = self.inner.read();
        for node in &inner.nodes {
            msgs.extend(node.to_messages());
        }
        msgs
    }
    pub fn push_event(&self, event: E) {
        let mut size = 0;
        {
            let msg = event.to_message();
            let role = msg.role();
            size += msg.size();
            let (text, _) = msg.to_tuple();
            let s = format!("\n{:?}:\n{}\n", role, text);
            let mut log_file = self.log_file.lock();
            log_file.write(s.as_bytes()).unwrap();
            log_file.flush().unwrap();
        }
        let mut inner = self.inner.write();
        inner.content_len += size;
        let node: &mut MindNode<E> = inner.last_mut().unwrap();
        node.push(event);
    }

    pub fn push_node(&self, node: MindNode<E>) {
        {
            let s = format!("\n{:?}:\n{}\n", Role::Assistant, &node.statement());
            let mut log_file = self.log_file.lock();
            log_file.write(s.as_bytes()).unwrap();
            log_file.flush().unwrap();
        }
        let mut inner = self.inner.write();

        //
        inner.back_to(node.node.pos);
        inner.nodes.push(node);
    }
    pub fn top_node(&self) -> Option<Node> {
        let inner = self.inner.read();
        inner.last().map(|v| v.node.clone())
    }
    pub fn push(&self, push: Node) {
        let mut inner = self.inner.write();
        inner.push(push);
    }
}
