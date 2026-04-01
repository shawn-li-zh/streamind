use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionResponseStream,
};
use tracing::error;

use crate::{
    event::AgentEvent,
    llm_engine::LLMEngine,
    stream::inlet::{ShareInlet, StreamCommand},
};
use futures::StreamExt;

mod inlet;

pub struct LLMStream {
    pub llm_engine: LLMEngine,
    pub messages: Vec<ChatCompletionRequestMessage>,
    pub buffer: String,
    pub is_runing: bool,
    pub is_stopped: bool,
}

impl LLMStream {
    pub fn load() {

        //@todo 从数据库加载
    }

    pub fn new(engine: LLMEngine, messages: Vec<ChatCompletionRequestMessage>) -> Self {
        LLMStream {
            llm_engine: engine,
            messages,
            buffer: String::new(),
            is_runing: false,
            is_stopped: false,
        }
    }

    pub async fn start(&mut self, mut inlet: ShareInlet) -> ShareInlet {
        self.is_runing = true;
        let stream = self.llm_engine.stream(self.messages.clone()).await.unwrap();
        inlet = self.chunk_loop(stream, inlet).await;
        inlet.borrow_mut().end();
        self.is_runing = false;
        return inlet;
    }

    async fn chunk_loop(
        &mut self,
        mut stream: ChatCompletionResponseStream,
        mut inlet: ShareInlet,
    ) -> ShareInlet {
        while let Some(result) = stream.next().await {
            if self.is_stopped {
                break;
            }
            if result.is_err() {
                error!("Stream error: {:?}", result.as_ref().err());
                break;
            }
            let response = result.unwrap();
            let choice = &response.choices[0];
            if let Some(chunk) = &choice.delta.content {
                self.buffer.push_str(chunk);

                let cmds = inlet.borrow_mut().input(chunk);
                for cmd in cmds {
                    match cmd {
                        StreamCommand::Next(inl) => inlet = inl,
                        StreamCommand::Stop() => {
                            break;
                        }
                        StreamCommand::Append(msgs) => {
                            self.messages.extend(msgs);
                            //重新创建流
                            stream = self.llm_engine.stream(self.messages.clone()).await.unwrap();
                        }
                        StreamCommand::Out(c) => todo!(),
                    }
                }
            }
        }
        self.stop();
        return inlet;
    }

    pub fn read(&self) -> String {
        let mut content = self.buffer.clone();
        if self.is_runing {
            content.push_str("[中断]");
        }
        return content;
    }
    pub fn stop(&mut self) {
        if self.is_stopped {
            return;
        }
        let response = self.read();
        self.is_stopped = true;
        self.trigger_event.response = response;
    }
}

fn build_messages(events: &Vec<AgentEvent>) -> Vec<ChatCompletionRequestMessage> {
    //事件转消息
    let content = (&events)
        .iter()
        .map(|e| e.to_message())
        .collect::<Vec<_>>()
        .join("\n\n");
    let msg = ChatCompletionRequestSystemMessageArgs::default()
        .content(content)
        .build()
        .unwrap();
    let event_msg = ChatCompletionRequestMessage::System(msg);

    return vec![event_msg];
}
pub struct MindStream {
    id: u32,
    upstream_id: u32,
    events: Vec<AgentEvent>,
    trigger_event: AgentEvent,
    llm_stream: LLMStream,
}

impl MindStream {
    pub fn new(
        id: u32,
        engine: LLMEngine,
        events: Vec<AgentEvent>,
        trigger_event: AgentEvent,
    ) -> Self {
        MindStream {
            id: id,
            upstream_id: 0,
            llm_stream: LLMStream::new(engine, build_messages(&events)),
            events:events,
            trigger_event: trigger_event,
        }
    }
    pub fn read(&self) -> String {
        let mut content = self.llm_stream.buffer.clone();
        if self.llm_stream.is_runing {
            content.push_str("[中断]");
        }
        return content;
    }
    //从当前状态创建分支
    pub fn fork(&self, id: u32, trigger_event: AgentEvent) -> Self {
        let mut events = self.events.clone();
        let mut old = self.trigger_event.clone();
        old.response = self.read();
        events.push(old);
        let stream = LLMStream::new(self.llm_stream.llm_engine.clone(), build_messages(&events));
        MindStream {
            id: id,
            upstream_id: self.id,
            llm_stream: stream,
            events: events,
            trigger_event: trigger_event,
        }
    }
}
