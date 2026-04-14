use std::collections::{HashMap, HashSet};

use anyhow::Ok;
use chrono::{DateTime, Local};
use sea_orm::EntityTrait;
use tokio::sync::{mpsc, watch};
pub mod event;
pub mod global;
pub mod section;
use crate::{
    agent::{
        event::AgentEventData,
        section::{Section, SectionEvent},
    },
    entity::{self, EventEntity, EventModel},
    global::AGENT_GLOBAL_CONTEXT,
    llm_engine::LLMEngine,
    stream::MindStream,
    tool::ToolExcutor,
};

use event::AgentEvent;

pub struct AgentGlobalContext {
    pub agent: HashMap<u64, AgentContext>,
    pub engine: HashMap<String, LLMEngine>,
}
pub struct AgentContext {
    info: entity::agent::Model,
}

pub struct Agent {
    pub id: u32,
    pub info: entity::agent::Model,
    tool_excutor: ToolExcutor,
    mind_stream: MindStream,
    event_sender: mpsc::Sender<AgentEvent>,
    section_receiver: mpsc::Receiver<SectionEvent>,
    watcher: (watch::Sender<bool>, watch::Receiver<bool>),
}
impl Agent {
    pub fn new(id: u32, info: entity::agent::Model, tool_packages: HashSet<String>) -> Agent {
        //通道
        let event = mpsc::channel(100);
        let section = mpsc::channel(100);
        let watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        //流
        let engine = AGENT_GLOBAL_CONTEXT.engine.get("default").unwrap().clone();
        let mind_stream = MindStream::new(engine, event.1, section.0);
        //工具执行器
        let tool_excutor = ToolExcutor {
            packages: tool_packages,
        };
        Agent {
            id,
            info,
            event_sender: event.0,
            section_receiver: section.1,
            watcher,
            mind_stream,
            tool_excutor,
        }
    }
    pub async fn send_event(
        &mut self,
        event_data: AgentEventData,
        force: bool,
    ) -> anyhow::Result<()> {
        self.watcher.1.wait_for(|&is_paused| !is_paused).await?;
        //

        self.send_event_pure(event_data, force).await
    }
    async fn send_event_pure(
        &mut self,
        event_data: AgentEventData,
        force: bool,
    ) -> anyhow::Result<()> {
        //@todo 保存数据库
        let event = AgentEvent {
            id: 0,
            agent_id: self.id,
            data: event_data,
            force,
            time: Local::now(),
        };
        let mut event_model = EventModel::default();
        event_model.agent_id = event.agent_id;
        event_model.event_type=event.data.name().to_string();
        event_model.force=event.force;
        event_model.time=event.time;
        EventEntity::insert(event_model.into()).exec(db).await;
        if let AgentEventData::Interrupt(reason) = &event.data {
            self.mind_stream.stop(reason.as_str()).await;
            return Ok(());
        }
        self.event_sender.send(event).await?;
        Ok(())
    }

    pub async fn start(&mut self) {
        loop {
            self.react_loop().await;
        }
    }

    async fn react_loop(&mut self) {
        let mut sections = vec![];
        let mut tool_calls = vec![];
        loop {
            if let Some(section_event) = self.section_receiver.recv().await {
                match section_event {
                    SectionEvent::Start => todo!(),
                    SectionEvent::Open(section_tag) => todo!(),
                    SectionEvent::Close(section) => {
                        //保存数据库+发送到前端
                        if let Section::ToolCall(code) = &section {
                            tool_calls.push(code.to_string());
                        }
                        sections.push(section);
                    }
                    SectionEvent::End => {
                        //如果有函数调用，则执行函数调用，然后发送事件继续循环，
                        //没有的话，则等待外部事件触发循环
                        if tool_calls.is_empty() {
                            break;
                        }
                        //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                        self.watcher.0.send(true).unwrap();
                        let results = self.tool_excutor.batch_call(tool_calls).await;
                        let results = results
                            .into_iter()
                            .map(|res| {
                                let r = match res {
                                    Ok(e) => format!("Ok:{}", e),
                                    Err(e) => format!("Error:{:?}", e),
                                };
                                r
                            })
                            .collect::<Vec<String>>();
                        let _ = self
                            .send_event_pure(AgentEventData::ToolCallResult(results), true)
                            .await
                            .unwrap();
                        self.watcher.0.send(false).unwrap();
                        break;
                    }
                }
                continue;
            }
            //
            return;
        }
    }
}
