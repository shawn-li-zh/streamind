use std::sync::RwLock;

use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};

use crate::{
    agent::{
        event::{AgentEvent, AgentEventData},
        section::{section_stream, Section, SectionEvent, SectionListener},
    },
    llm_engine::LLMEngine,
};
use futures::{Stream, StreamExt};

mod inlet;
mod parser;
fn to_message(event: &AgentEvent) -> ChatCompletionRequestMessage {
    match &event.data {
        crate::agent::event::AgentEventData::UserInput(_) => {
            let msg = ChatCompletionRequestUserMessageArgs::default()
                .content(event.to_message())
                .build()
                .unwrap();
            ChatCompletionRequestMessage::User(msg)
        }
        _ => {
            let msg = ChatCompletionRequestSystemMessageArgs::default()
                .content(event.to_message())
                .build()
                .unwrap();
            ChatCompletionRequestMessage::System(msg)
        }
    }
}
fn build_messages(events: &Vec<AgentEvent>) -> Vec<ChatCompletionRequestMessage> {
    let mut msgs = vec![];
    //事件转消息
    for ele in events {
        let msg = to_message(ele);
        msgs.push(msg);
        if !ele.response.is_empty() {
            let msg = ChatCompletionRequestAssistantMessageArgs::default()
                .content(ele.response.to_string())
                .build()
                .unwrap();
            let msg = ChatCompletionRequestMessage::Assistant(msg);
            msgs.push(msg);
        }
    }
    return msgs;
}

use futures::stream;
use tokio::sync::{mpsc, oneshot};

pub struct MindStream {
    pub event_buffer: Vec<AgentEvent>,
    pub messages: Vec<ChatCompletionRequestMessage>,
    llm_engine: LLMEngine,
    is_running: bool,

    // 用于向流中注入“中断标签”的发送器
    stop_tx: Option<mpsc::UnboundedSender<String>>,
    // 用于等待 run() 完全退出的接收器
    done_rx: Option<oneshot::Receiver<()>>,
    last_section_head: RwLock<String>,
    receiver: mpsc::Receiver<AgentEvent>,
    sender: mpsc::Sender<SectionEvent>,
}

impl MindStream {
    pub fn new(
        engine: LLMEngine,
        receiver: mpsc::Receiver<AgentEvent>,
        sender: mpsc::Sender<SectionEvent>,
    ) -> Self {
        MindStream {
            llm_engine: engine,
            messages: vec![],
            event_buffer: vec![],
            is_running: false,
            stop_tx: None,
            done_rx: None,
            last_section_head: RwLock::new(String::new()),
            receiver,
            sender,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }
    async fn chunk_stream(&self) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
        let llm_stream = self.llm_engine.stream(self.messages.clone()).await.unwrap();
        llm_stream
            .map(|c| match c {
                // 优化：使用 first_mut 和 and_then 扁平化取值，避免繁杂的 if/else
                Ok(mut r) => r
                    .choices
                    .first_mut()
                    .and_then(|choice| choice.delta.content.take())
                    .unwrap_or_default(),
                Err(e) => {
                    let section = format!("<error>Stream error: {:?}</error>", e);
                    // &self.inject_section(&section);
                    // String::new()
                    section
                }
            })
            .boxed()
    }

    /// 停止流
    pub async fn stop(&mut self, info: &str) {
        if !self.is_running {
            return;
        }
        self.is_running = false;

        if let Some(rx) = self.done_rx.take() {
            //阻塞等待完全停止
            let _ = rx.await.unwrap();
        }
        // 1. 给正在处理中的流插入一条中断事件内容
        let section = format!("<interupt>{}</interupt>", info);
        self.inject_section(&section);
        // self.stop_channel.0.send(section).unwrap();
    }
    fn inject_section(&self, section: &str) {
        let mut last_section_head = self.last_section_head.write().unwrap();
        let head = std::mem::replace(&mut *last_section_head, String::new());
        let end = format!("</{}>", head);
        if let Some(tx) = &self.stop_tx {
            tx.send(format!("{}{}", end, section)).unwrap();
        }
    }
    /// 核心方法：非阻塞地抽干(Drain)当前队列中的所有事件
    fn drain_events(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event.data {
                AgentEventData::Interrupt(_) => {
                    // 在 drain 阶段遇到打断信号可以直接忽略
                    // 因为我们本身就是在打断后的重新整理阶段
                    println!("  [队列提取] 忽略多余的打断信号");
                }
                _ => {
                    self.event_buffer.push(event);
                }
            }
        }
    }
    async fn start(&mut self) {
        self.is_running = true;
        self.section_loop().await;
        self.is_running = false;
    }
    async fn section_loop(&mut self) {
        let mut is_interupt = false;
        loop {
            // 1. 创建注入通道 (mpsc) 和 完成信号通道 (oneshot)
            let (tx, rx) = mpsc::unbounded_channel::<String>();
            let (done_tx, done_rx) = oneshot::channel();
            self.stop_tx = Some(tx);
            self.done_rx = Some(done_rx);
            // 1. 如果没有待处理任务，阻塞等待下一个事件到达
            if self.event_buffer.is_empty() {
                let opt = self.receiver.recv().await;
                if opt.is_none() {
                    println!("通道已关闭，智能体即将退出");
                    break;
                }
                let event = opt.unwrap();
                self.event_buffer.push(event);
            }

            // 3. 准备当前要处理的上下文
            // 提取出当前所有的任务，清空 pending 准备接收新的
            let buffer = std::mem::take(&mut self.event_buffer);
            let msgs = build_messages(&buffer);
            self.messages.extend(msgs);
            // 4. 创建生成流（模拟大模型输出或耗时任务）
            let chunk_stream = self.chunk_stream().await;
            // 3. 将 Rx 转换为 Stream 并与 LLM 流合并
            let inject_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed();

            let merged_stream = stream::select(chunk_stream, inject_stream).boxed();

            let mut section_stream = Box::pin(section_stream(merged_stream));

            // 5. 边生成边监听队列
            loop {
                tokio::select! {
                        // 分支 A：段落生成流
                    section_event_opt = section_stream.next() =>  {
                        if let Some(event)=section_event_opt{

                            match &event{
                                SectionEvent::Close(section)=>{
                                    if matches!(section,Section::Interrupt(_)){
                                        is_interupt=true;
                                    }
                                }
                                SectionEvent::End=>{
                                     is_interupt=true;
                                }
                                _=>{}
                            }
                            self.sender.send(event);

                     }else{
                                return;
                            }

                        }

                        // 分支 B：在生成过程中，收到了新事件
                        event_opt = self.receiver.recv() => {
                            if let Some(event)=event_opt{
                                    if event.force{
                                        self.sender.send(SectionEvent::End);
                                        break; // 跳出内部 loop,强制处理
                                    }
                                        // 如果是普通任务，先默默存起来，不打断当前生成
                                self.event_buffer.push(event)
                            }else{
                                return;
                            }
                        }
                }
                if is_interupt {
                    done_tx.send(()).unwrap();
                    self.stop_tx = None;
                    self.done_rx = None;
                    return;
                }
            }
            // 2. 瞬间抽干队列中同时到达的其他任务（Batching）
            self.drain_events();
        }
    }
}
