use std::{
    backtrace::Backtrace,
    collections::HashMap,
    fs::File,
    io::Write,
    sync::{Arc, LazyLock},
};

use entity::event::{EnvEventStatus, EnvEventType};
use entity_service::DB;
use futures::{Stream, StreamExt, stream};

use kernel::{
    agent::AgentEvent,
    message::{Message, Role},
    section::{self, Section},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    kit::{message_kit, prompt_kit},
    llm::client::{ChatClient, LLMClient},
};

/// test
static OUTPUTS: LazyLock<Vec<Vec<String>>> = LazyLock::new(|| {
    if let Ok(test) = std::fs::read_to_string("./test.xml") {
        return test
            .split("<end/>")
            .filter(|v| !v.is_empty())
            .map(|item| split_into_random_chunks(item, 2, 5))
            .collect();
    }
    vec![]
});

fn split_into_random_chunks(input: &str, min_size: usize, max_size: usize) -> Vec<String> {
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
pub struct MindStack {
    pub log_file: File,
    pub system_prompt: Message,
    pub nodes: Vec<MindNode>,
    id_map_pos: HashMap<i64, usize>,
    //上下文长度
    pub content_len: usize,
    step: usize,
}
impl MindStack {
    pub fn new(system_prompt: String, log_file: File) -> Self {
        Self {
            log_file,
            system_prompt: Role::system().text(system_prompt),
            nodes: vec![],
            id_map_pos: HashMap::new(),
            content_len: 0,
            step: 0,
        }
    }
    pub fn push_events(&mut self, event_buffer: &mut Vec<AgentEvent>) {
        info!("received events [{}]", event_buffer.len());
        let buffer = std::mem::take(event_buffer);
        for event in buffer {
            let msg = event.to_message();
            self.push_msg(msg);
        }
    }
    pub fn push_response(&mut self, sections: Vec<Section>) {
        if let Some(output) = message_kit::to_assistant_ouput(sections) {
            let msg = Role::assistant().text(output);
            self.push_msg(msg);
        }
        self.step += 1;
    }

    pub fn push(&mut self, push: Push) {
        let node = MindNode {
            id: push.id,
            pos: push.pos,
            title: push.title,
            content: push.content,
            messages: vec![],
            content_len: 0,
        };
        self.id_map_pos.insert(node.id, self.nodes.len());
        self.back_to(push.pos);
        self.nodes.push(node);
    }

    pub fn push_node(&mut self, node: MindNode) {
        self.id_map_pos.insert(node.id, node.pos);
        self.back_to(node.pos);
        self.nodes.push(node);
    }
    //回到指定位置，并丢弃后面的节点
    fn back_to(&mut self, pos: usize) -> Vec<MindNode> {
        let removed = self.nodes.split_off(pos);
        for ele in &removed {
            self.content_len -= ele.content_len;
        }
        return removed;
    }
    pub fn push_msg(&mut self, msg: Message) {
        self.content_len += msg.size;
        // let s = format!("{:?}\n\n", &msg);
        // self.log_file.write(s.as_bytes()).unwrap();
        // self.log_file.flush().unwrap();
        let node: &mut MindNode = self.last_mut().unwrap();
        node.push(msg);
    }
    fn last_ref(&self) -> Option<&MindNode> {
        self.nodes.last()
    }
    fn last_mut(&mut self) -> Option<&mut MindNode> {
        self.nodes.last_mut()
    }
    fn messages(&self) -> Vec<Message> {
        let mut msgs = vec![self.system_prompt.clone()];
        for node in &self.nodes {
            msgs.extend(node.to_messages());
        }
        msgs
    }
    // pub async fn chunk_stream(
    //     &self,
    //     llm_engine: Arc<dyn ChatClient>,
    // ) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
    //     let messages = self.messages();
    //     // println!("{:#?}", &messages);
    //     let llm_stream = llm_engine
    //         .stream(messages)
    //         .await
    //         .expect("Failed to llm stream");

    //     llm_stream
    //         .map(|r| match r {
    //             Ok(v) => v,
    //             Err(e) => {
    //                 let a = Backtrace::capture();
    //                 eprintln!("{:#?}", a);
    //                 format!("<error>{e}</error>")
    //             }
    //         })
    //         .boxed()
    // }

    pub async fn chunk_stream(
        &self,
        llm_engine: Arc<dyn ChatClient>,
    ) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
        let o = &*OUTPUTS;
        let i = self.step % o.len();
        stream::iter(o[i].clone()).boxed()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Push {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
}
pub struct MindNode {
    pub id: i64,
    pub pos: usize,
    pub title: String,
    pub content: String,
    pub messages: Vec<Message>,
    //上下文长度
    pub content_len: usize,
}
impl MindNode {
    pub fn to_messages(&self) -> Vec<Message> {
        let mut msgs = Vec::with_capacity(self.messages.len() + 1);
        //里面的内容都是第一人称
        let state = Role::system().text(format!(
            r#"<栈节点>
            {}:{}
            {}
            </栈节点>"#,
            self.pos, self.title, self.content
        ));
        msgs.push(state);
        msgs.extend(self.messages.clone());
        msgs
    }

    pub fn push(&mut self, msg: Message) {
        self.content_len += msg.size;
        self.messages.push(msg);
    }
}
