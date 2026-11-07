use std::sync::{Arc, LazyLock, atomic::AtomicUsize};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use kernel::{
    client::ChatClient,
    message::{Role, RoleMessage},
    mind::context::{ChunkStream, EventContext},
    mind::event::{self, EnvEvent},
};

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// test
// pub(crate) static OUTPUTS: LazyLock<Vec<Vec<String>>> = LazyLock::new(|| {
//     if let Ok(test) = std::fs::read_to_string("./test.xml") {
//         return test
//             .split("<end/>")
//             .map(|item| item.trim())
//             .filter(|v| !v.is_empty())
//             .map(|item| split_into_chunks(item))
//             .collect();
//     }
//     vec![]
// });

// pub fn split_into_chunks(input: &str) -> Vec<String> {
//     let chars: Vec<char> = input.chars().collect(); // 转为字符数组
//     let mut chunks = Vec::new();
//     let mut start = 0;
//     let len = chars.len();

//     while start < len {
//         // 随机决定当前块的长度
//         let chunk_size = 2;
//         let end = (start + chunk_size).min(len);

//         // 获取切片并转回 String
//         let chunk: String = chars[start..end].iter().collect();
//         chunks.push(chunk);

//         start = end;
//     }

//     chunks
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
}
impl Node {
    pub fn statement(&self) -> String {
        format!(
            r#"(stack pos={} title="{}"\n{})"#,
            self.pos,
            self.title,
            self.content
        )
    }
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
            r#"(stack pos={} title="{}"\n{})"#,
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
        // 事件已清空，size 必须一起归零；否则之后 pop 时会按旧值重复扣减，
        // 导致 StackContext::content_size 下溢成天文数字，触发「上下文溢出」中断
        self.content_size
            .store(0, std::sync::atomic::Ordering::Relaxed);
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
    /// 重置栈顶节点：先把该节点占用的体积从上下文里扣掉，再清空事件
    pub fn reset_top_node(&self, content: String) -> Option<Arc<MindNode<E>>> {
        let node = self.top_node()?;
        let size = node.size();
        let current = self.content_size.load(std::sync::atomic::Ordering::Relaxed);
        self.content_size.store(
            current.saturating_sub(size),
            std::sync::atomic::Ordering::Relaxed,
        );
        node.reset(content);
        Some(node)
    }

    pub fn pop_node(&self) -> Option<Arc<MindNode<E>>> {
        let mut nodes = self.nodes.write();
        if let Some(node) = nodes.pop() {
            let size = node.size();
            // 用饱和减法而不是 fetch_sub：计数值万一偏小也不会下溢成 usize::MAX
            let current = self.content_size.load(std::sync::atomic::Ordering::Relaxed);
            self.content_size.store(
                current.saturating_sub(size),
                std::sync::atomic::Ordering::Relaxed,
            );
            return Some(node);
        }
        None
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

    pub fn backup(&self) -> Vec<(Node, Vec<Arc<E>>)> {
        let inner = self.nodes.read();
        inner
            .iter()
            .map(|node| {
                let n = Node {
                    id: node.id,
                    pos: node.pos,
                    title: node.title.clone(),
                    content: node.content.read().clone(),
                };
                let events = node.events();
                (n, events)
            })
            .collect()
    }
    pub fn node_len(&self) -> usize {
        return self.nodes.read().len();
    }
}

#[async_trait]
impl<E: EnvEvent> EventContext<E> for StackContext<E> {
    fn push_event<T: Into<E>>(&self, event: T) {
        let event = event.into();
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
    async fn chunk_stream(&self) -> anyhow::Result<ChunkStream> {
        let messages = self.messages();
        let llm_stream = self.client.stream(messages).await?;
        Ok(llm_stream)
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
#[cfg(test)]
mod tests {
    use super::*;
    use kernel::mind::context::EventContext;
    use kernel::model::{ModelFamily, ModelModal};

    struct DummyClient;

    #[async_trait]
    impl ChatClient for DummyClient {
        fn model_info(&self) -> (ModelFamily, String, Vec<ModelModal>) {
            (ModelFamily::OpenAI, "dummy".to_string(), vec![])
        }
        async fn stream(&self, _messages: Vec<RoleMessage>) -> anyhow::Result<ChunkStream> {
            anyhow::bail!("测试环境不应真的调用模型")
        }
    }

    #[derive(Clone)]
    struct DummyEvent(String);

    impl EnvEvent for DummyEvent {
        fn to_message(&self) -> RoleMessage {
            RoleMessage::System(vec![self.0.clone().into()])
        }
        fn size(&self) -> usize {
            self.0.len()
        }
    }

    fn node(id: i64, pos: usize) -> Node {
        Node {
            id,
            pos,
            title: format!("node-{id}"),
            content: String::new(),
        }
    }

    fn stack() -> StackContext<DummyEvent> {
        StackContext::new(Arc::new(DummyClient), "system".to_string())
    }

    #[test]
    fn reset_clears_node_size() {
        let n = MindNode::new(node(1, 0));
        n.push(DummyEvent("hello".into()));
        n.push(DummyEvent("world".into()));
        assert_eq!(n.size(), 10);
        assert_eq!(n.len(), 2);

        n.reset("重置".to_string());

        assert_eq!(n.size(), 0, "reset 之后 size 必须归零");
        assert_eq!(n.len(), 0);
        assert_eq!(n.content.read().as_str(), "重置");
    }

    #[test]
    fn stack_size_accounting_survives_reset_and_pop() {
        let stack = stack();
        stack.push_node(node(1, 0));
        stack.push_event(DummyEvent("abc".into()));
        assert_eq!(stack.size(), 3);

        stack.push_node(node(2, 1));
        stack.push_event(DummyEvent("defg".into()));
        assert_eq!(stack.size(), 7);

        stack.reset_top_node("重置".to_string());
        assert_eq!(stack.size(), 3, "reset 必须扣减栈顶节点占用的体积");

        stack.pop_node();
        assert_eq!(stack.size(), 3);

        stack.pop_node();
        assert_eq!(stack.size(), 0);
    }

    #[test]
    fn pop_never_underflows() {
        let stack = stack();
        stack.push_node(node(1, 0));
        stack.push_event(DummyEvent("abc".into()));
        // 人为把计数改小，模拟历史统计偏差
        stack
            .content_size
            .store(0, std::sync::atomic::Ordering::Relaxed);
        stack.pop_node();
        assert_eq!(stack.size(), 0, "计数不能下溢成 usize::MAX");
    }
}
