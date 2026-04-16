use std::sync::{Arc, RwLock};

use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};

use crate::agent::{
    event::{AgentEvent, AgentEventData},
    llm_engine::LLMEngine,
    section::{section_stream, Section, SectionEvent},
};

use futures::{Stream, StreamExt};

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
fn build_bundle_messages(events: &Vec<AgentEvent>) -> ChatCompletionRequestMessage {
    //
    let mut msgs = vec![];
    //事件转消息
    for ele in events {
        let msg = ele.to_message();
        msgs.push(msg);
    }
    let c = msgs.join("\n\n");
    let msg = ChatCompletionRequestSystemMessageArgs::default()
        .content(c)
        .build()
        .unwrap();
    ChatCompletionRequestMessage::System(msg)
}

use futures::stream;
use tokio::sync::{mpsc, oneshot, watch};

pub struct MindStream {
    pub event_buffer: Vec<AgentEvent>,
    pub messages: Vec<ChatCompletionRequestMessage>,
    llm_engine: Arc<LLMEngine>,
    is_running: bool,
    last_section_head: RwLock<String>,
    inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    event_receiver: mpsc::Receiver<AgentEvent>,
    section_sender: mpsc::Sender<SectionEvent>,
    run_watcher: watch::Sender<bool>,
}

impl MindStream {
    pub fn new(
        engine: Arc<LLMEngine>,
        system_prompt: String,
        inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
        event_receiver: mpsc::Receiver<AgentEvent>,
        section_sender: mpsc::Sender<SectionEvent>,
        run_watcher: watch::Sender<bool>,
    ) -> Self {
        let msg = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
            .unwrap();
        MindStream {
            llm_engine: engine,
            messages: vec![ChatCompletionRequestMessage::System(msg)],
            event_buffer: vec![],
            is_running: false,
            last_section_head: RwLock::new(String::new()),
            inject_receiver,
            event_receiver,
            section_sender,
            run_watcher,
        }
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

    fn inject_section(
        &self,
        inject_sender: &mpsc::UnboundedSender<String>,
        injected: (String, bool),
    ) {
        let (inject, is_section) = injected;
        if is_section {
            let mut last_section_head = self.last_section_head.write().unwrap();
            let head = std::mem::replace(&mut *last_section_head, String::new());
            let end = format!("</{}>", head);
            inject_sender.send(format!("{}{}", end, inject)).unwrap();
            return;
        }
        //普通内容
        inject_sender.send(inject).unwrap();
    }
    /// 核心方法：非阻塞地抽干(Drain)当前队列中的所有事件
    fn drain_events(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
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
    pub async fn start(&mut self) {
        self.section_loop().await;
        self.is_running = false;
        self.run_watcher.send(false).unwrap();
    }
    async fn section_loop(&mut self) {
        loop {
            self.run_watcher.send(true).unwrap();
            // 1. 如果没有待处理任务，阻塞等待下一个事件到达
            if self.event_buffer.is_empty() {
                let opt = self.event_receiver.recv().await;
                if opt.is_none() {
                    // println!("通道已关闭，智能体即将退出");
                    break;
                }
                let event = opt.unwrap();
                self.event_buffer.push(event);
            }
            self.is_running = true;
            let proxy = mpsc::unbounded_channel();
            // 3. 准备当前要处理的上下文
            // 提取出当前所有的任务，清空 pending 准备接收新的
            let buffer = std::mem::take(&mut self.event_buffer);
            let msg = build_bundle_messages(&buffer);
            self.messages.push(msg);
            // 4. 创建生成流（模拟大模型输出或耗时任务）
            let chunk_stream = self.chunk_stream().await;
            // 3. 将 Rx 转换为 Stream 并与 LLM 流合并
            let inject_stream =
                tokio_stream::wrappers::UnboundedReceiverStream::new(proxy.1).boxed();

            let merged_stream = stream::select(chunk_stream, inject_stream).boxed();

            let mut section_stream = Box::pin(section_stream(merged_stream));

            // 5. 边生成边监听队列
            loop {
                tokio::select! {
                    Some(injected)=self.inject_receiver.recv() =>{
                        self.inject_section(&proxy.0, injected);
                    }
                        // 分支 A：段落生成流
                     Some(section_event) = section_stream.next() =>  {
                            let is_end= matches!(&section_event,SectionEvent::End);
                            self.section_sender.send(section_event).await.unwrap();
                            if is_end{
                                return ;
                            }
                        }

                        // 分支 B：在生成过程中，收到了新事件
                        Some(event) = self.event_receiver.recv() => {
                           if event.force{
                            self.event_buffer.push(event);
                                        self.section_sender.send(SectionEvent::End).await.unwrap();
                                        break; // 跳出内部 loop,强制处理
                                    }
                                        // 如果是普通任务，先默默存起来，不打断当前生成
                                self.event_buffer.push(event)
                        }
                        else=>{
                            break ;
                        }
                };
            }

            // 2. 瞬间抽干队列中同时到达的其他任务（Batching）
            self.drain_events();
        }
    }
}
