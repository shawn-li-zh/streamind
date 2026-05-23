use std::{
    backtrace::Backtrace,
    convert::Infallible,
    fs::File,
    io::Write,
    sync::{Arc, LazyLock},
};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio::sync::RwLock;

use crate::{
    client::ChatClient,
    message::{Role, RoleMessage},
    mind::{context::EventContext, env::MindEvent},
};
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
struct StackContextInner<E: Sync> {
    pub nodes: Vec<MindNode<E>>,
    //上下文长度
    pub content_len: usize,
}
impl<E: Sync> StackContextInner<E> {
    pub fn new() -> Self {
        Self {
            nodes: vec![],

            content_len: 0,
        }
    }

    pub fn push(&mut self, push: Push) {
        let node = MindNode {
            id: push.id,
            pos: push.pos,
            title: push.title,
            content: push.content,
            events: vec![],
            content_len: 0,
        };

        self.back_to(push.pos);
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

pub struct StackContext<E: Sync> {
    pub system_prompt: RoleMessage,
    pub log_file: RwLock<File>,
    pub client: Arc<dyn ChatClient>,
    inner: RwLock<StackContextInner<E>>,
}
impl<E: Sync> StackContext<E> {
    pub fn new(client: Arc<dyn ChatClient>, system_prompt: String, log_file: File) -> Self {
        Self {
            client,
            log_file: RwLock::new(log_file),
            system_prompt: RoleMessage::System(vec![system_prompt.into()]),
            inner: RwLock::new(StackContextInner::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Push {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
}
pub struct MindNode<E> {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
    pub events: Vec<E>,
    //上下文长度
    pub content_len: usize,
}
impl<E: MindEvent> MindNode<E> {
    pub fn statement(&self) -> String {
        format!(
            "<push pos={}>{}\n\n{}\n</push>",
            self.pos, self.title, self.content
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
impl<E: MindEvent> EventContext<E> for StackContext<E> {
    async fn push_events(&self, events: Vec<E>) {
        info!("received events [{}]", events.len());
        for event in events {
            self.push_event(event);
        }
    }

    async fn chunk_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>> {
        let messages = self.messages().await;
        // println!("{:#?}", &messages);
        let llm_stream = self
            .client
            .stream(messages)
            .await
            .expect("Failed to llm stream");

        llm_stream
    }
}
impl<E: MindEvent> StackContext<E> {
    async fn messages(&self) -> Vec<RoleMessage> {
        let mut msgs = vec![self.system_prompt.clone()];
        let inner = self.inner.read().await;
        for node in &inner.nodes {
            msgs.extend(node.to_messages());
        }
        msgs
    }
    pub async fn push_event(&self, event: E) {
        let mut size = 0;
        {
            let msg = event.to_message();
            let role = msg.role();
            size += msg.size();
            let (text, _) = msg.to_tuple();
            let s = format!("\n{:?}:\n{}\n", role, text);
            let mut log_file = self.log_file.write().await;
            log_file.write(s.as_bytes()).unwrap();
            log_file.flush().unwrap();
        }
        let mut inner = self.inner.write().await;
        inner.content_len += size;
        let node: &mut MindNode<E> = inner.last_mut().unwrap();
        node.push(event);
    }

    pub async fn push_node(&self, node: MindNode<E>) {
        {
            let s = format!("\n{:?}:\n{}\n", Role::Assistant, &node.statement());
            let mut log_file = self.log_file.write().await;
            log_file.write(s.as_bytes()).unwrap();
            log_file.flush().unwrap();
        }
        let mut inner = self.inner.write().await;

        //
        inner.back_to(node.pos);
        inner.nodes.push(node);
    }

    pub async fn push(&self, push: Push) {
        let mut inner = self.inner.write().await;
        inner.push(push);
    }
}
