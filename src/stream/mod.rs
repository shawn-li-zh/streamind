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
    pub id: u32,
    pub upstream_id: u32,
    pub llm_engine: LLMEngine,
    pub events: Vec<AgentEvent>,
    pub trigger_event: AgentEvent,
    pub buffer: String,
    pub is_runing: bool,
    pub is_stopped: bool,
}

impl LLMStream {
    pub fn load() {

        //@todo 从数据库加载
    }

    pub fn new(id: u32, engine: LLMEngine, trigger_event: AgentEvent) -> Self {
        LLMStream {
            id: id,
            upstream_id: 0,
            llm_engine: engine,
            events: vec![],
            trigger_event: trigger_event,
            buffer: String::new(),
            is_runing: false,
            is_stopped: false,
        }
    }

    pub async fn start(&mut self, mut inlet: ShareInlet) -> ShareInlet {
        self.is_runing = true;
        let messages = self.build_messages();
        let stream = self.llm_engine.stream(messages).await.unwrap();
        inlet = self.chunk_loop(stream, inlet).await;
        inlet.borrow_mut().end();
        self.is_runing = false;
        return inlet;
    }
    fn build_messages(&self) -> Vec<ChatCompletionRequestMessage> {
        //事件转消息
        let content = (&self.events)
            .iter()
            .map(|e| e.to_message())
            .collect::<Vec<_>>()
            .join("\n\n");
        let msg = ChatCompletionRequestSystemMessageArgs::default()
            .content(content)
            .build()
            .unwrap();
        let event_msg = ChatCompletionRequestMessage::System(msg);
        //添加当前时间提示
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let time_msg = ChatCompletionRequestSystemMessageArgs::default()
            .content(format!("当前时间：{}", now))
            .build()
            .unwrap();
        let time_msg = ChatCompletionRequestMessage::System(time_msg);
        return vec![event_msg, time_msg];
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
            match result {
                Ok(response) => {
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
                                StreamCommand::Append(msg) => todo!(),
                                StreamCommand::Out(c) => todo!(),
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("{}", err);
                    break;
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
    //从当前状态创建分支
    pub fn fork(&self, id: u32, trigger_event: AgentEvent) -> Self {
        let mut events = self.events.clone();
        let mut old = self.trigger_event.clone();
        old.response = self.read();
        events.push(old);
        let mut stream = LLMStream::new(id, self.llm_engine.clone(), trigger_event);
        stream.events = events;
        stream
    }
}
