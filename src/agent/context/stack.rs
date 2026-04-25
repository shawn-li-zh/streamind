use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use async_openai::types::ChatCompletionRequestMessage;
use entity::event::EnvEventType;
use futures::{stream, Stream, StreamExt};
use rand::Rng;
use tracing::{error, info};

use crate::agent::{
    event::{to_type_enum, AgentEvent},
    kit::{message_kit, prompt_kit},
    llm_engine::LLMEngine,
};

/// test
static TEST: &str = include_str!("../../../test.xml");
static OUTPUTS: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let (d1, d2) = TEST.split_once("<end/>").unwrap();
    let d1 = split_into_random_chunks(d1, 2, 5);
    let d2 = split_into_random_chunks(d2, 2, 5);
    (d1, d2)
});

fn split_into_random_chunks(input: &str, min_size: usize, max_size: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = input.chars().collect(); // 转为字符数组
    let mut chunks = Vec::new();
    let mut start = 0;
    let len = chars.len();

    while start < len {
        // 随机决定当前块的长度
        let chunk_size = rng.gen_range(min_size..=max_size);
        let end = (start + chunk_size).min(len);

        // 获取切片并转回 String
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        start = end;
    }

    chunks
}
pub struct MindStack {
    pub nodes: Vec<MindNode>,
    id_map_pos: HashMap<i64, usize>,
    //上下文长度
    pub content_len: usize,
    last: EnvEventType,
}
impl MindStack {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            id_map_pos: HashMap::new(),
            content_len: 0,
            last: EnvEventType::AgentResponse,
        }
    }
    pub fn push_events(&mut self, event_buffer: &mut Vec<AgentEvent>) {
        let buffer = std::mem::take(event_buffer);
        if let Some(last) = buffer.last() {
            self.last = to_type_enum(&last.data);
        }
        let (msg, len) = message_kit::build_step_messages(&buffer);
        self.push_msg(msg, len);
    }
    pub fn push(&mut self, node: MindNode) {
        self.id_map_pos.insert(node.id, self.nodes.len());
        self.nodes.push(node);
    }
    //回到指定位置，并丢弃后面的节点
    pub fn back_to(&mut self, pos: usize) -> Vec<MindNode> {
        let removed = self.nodes.split_off(pos + 1);
        for ele in &removed {
            self.content_len -= ele.content_len;
        }
        return removed;
    }
    pub fn push_msg(&mut self, msg: ChatCompletionRequestMessage, len: usize) {
        self.content_len += len;
        let node = self.last_mut().unwrap();
        node.push(msg, len);
    }
    pub fn last_ref(&self) -> Option<&MindNode> {
        self.nodes.last()
    }
    pub fn last_mut(&mut self) -> Option<&mut MindNode> {
        self.nodes.last_mut()
    }
    fn messages(&self) -> Vec<ChatCompletionRequestMessage> {
        let mut msgs = vec![];
        for node in &self.nodes {
            msgs.extend(node.to_messages());
        }
        msgs
    }
    // pub async fn chunk_stream(
    //     &self,
    //     llm_engine: Arc<LLMEngine>,
    // ) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
    //     let messages = self.messages();
    //     let llm_stream = llm_engine.stream(messages).await.unwrap();
    //     llm_stream
    //         .map(|c| match c {
    //             // 优化：使用 first_mut 和 and_then 扁平化取值，避免繁杂的 if/else
    //             Ok(mut r) => r
    //                 .choices
    //                 .first_mut()
    //                 .and_then(|choice| choice.delta.content.take())
    //                 .unwrap_or_default(),
    //             Err(e) => {
    //                 let section = format!("<error>Stream error: {:?}</error>", &e);
    //                 error!("{}", e);
    //                 // &self.inject_section(&section);
    //                 // String::new()
    //                 section
    //             }
    //         })
    //         .boxed()
    // }

    pub async fn chunk_stream(
        &self,
        llm_engine: Arc<LLMEngine>,
    ) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
        let o = &*OUTPUTS;
        match &self.last {
            EnvEventType::UserRequest => stream::iter(o.0.clone()).boxed(),
            EnvEventType::ToolCallResult => stream::iter(o.1.clone()).boxed(),
            _ => stream::empty().boxed(),
        }
    }
}
pub struct MindNode {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub messages: Vec<ChatCompletionRequestMessage>,
    //上下文长度
    pub content_len: usize,
}
impl MindNode {
    pub fn to_messages(&self) -> Vec<ChatCompletionRequestMessage> {
        let mut msgs = Vec::with_capacity(self.messages.len() + 1);
        //里面的内容都是第一人称，所以用assistant
        let state = prompt_kit::assistant_message(format!(
            r#"<state id={} title="{}">{}</state>"#,
            self.id, self.title, self.content
        ));
        msgs.push(state);
        msgs.extend(self.messages.clone());
        msgs
    }

    pub fn push(&mut self, msg: ChatCompletionRequestMessage, len: usize) {
        self.messages.push(msg);
        self.content_len += len;
    }
}
