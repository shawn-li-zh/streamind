use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicI64, AtomicU32, Ordering},
        Arc,
    },
};

use async_openai::types::ChatCompletionRequestSystemMessageArgs;
use axum::response::sse;
use chrono::Local;
use entity::agent;
use futures::StreamExt;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tracing::info;

use crate::{
    agent::{
        event::{AgentEvent, AgentEventData, ToolCallResultItem},
        global::{AGENT_POOL, ENGINE_POOL},
        kit::{entity_kit, message_kit, prompt_kit},
        section::{Section, SectionEvent},
        stream::MindStream,
    },
    global::{DB, PROMPT_CACHE},
    tool::ToolExcutor,
};

pub struct Agent {
    pub info: entity::agent::Model,
    event_sender: mpsc::Sender<AgentEvent>,
    call_watcher: watch::Receiver<bool>,
    run_watcher: watch::Receiver<bool>,
    inject_sender: mpsc::UnboundedSender<(String, bool)>,
    pub web_sender: Option<mpsc::UnboundedSender<sse::Event>>,
}

pub struct AgentRuntime {
    pub agent: Arc<RwLock<Agent>>,
    tool_excutor: ToolExcutor,
    section_receiver: mpsc::Receiver<SectionEvent>,
    call_watcher: watch::Sender<bool>,
}

impl Agent {
    pub async fn new(info: entity::agent::Model) -> Arc<RwLock<Agent>> {
        //通道
        let event = mpsc::channel(100);
        let section = mpsc::channel(100);
        let inject = mpsc::unbounded_channel();
        let call_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        let run_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        // 加载历史消息
        let system_prompt = prompt_kit::build_system_prompt(&info).await;
        let mut messages = vec![ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
            .unwrap()
            .into()];
        let msgs = message_kit::load_messages(info.id, info.state_version)
            .await
            .unwrap();
        messages.extend(msgs);
        //流
        let p = ENGINE_POOL.read().await;
        let engine = p.get("default").unwrap().clone();
        let mut mind_stream = MindStream::new(
            engine,
            messages,
            inject.1,
            event.1,
            section.0,
            run_watcher.0,
        );
        tokio::spawn(async move {
            loop {
                mind_stream.start().await;
            }
        });
        //工具执行器
        let tool_packages: HashSet<String> = info.tool_packages.0.clone().into_iter().collect();
        let tool_excutor = ToolExcutor {
            packages: tool_packages,
        };
        let agent = Agent {
            info,
            event_sender: event.0,
            call_watcher: call_watcher.1,
            run_watcher: run_watcher.1,
            inject_sender: inject.0,
            web_sender: None,
        };
        let agent = Arc::new(RwLock::new(agent));
        let mut runtime = AgentRuntime {
            agent: agent.clone(),
            section_receiver: section.1,
            call_watcher: call_watcher.0,
            tool_excutor,
        };
        tokio::spawn(async move { runtime.react_loop().await.unwrap() });

        agent
    }
    pub async fn stop(&mut self, reason: String) -> anyhow::Result<()> {
        self.inject_sender
            .send((format!("<interrupt>{}</interrupt>", reason), true))?;
        self.run_watcher.wait_for(|v| !*v).await?;
        Ok(())
    }
    pub async fn send_event(
        &mut self,
        event_data: AgentEventData,
        force: bool,
    ) -> anyhow::Result<entity::event::Model> {
        self.call_watcher.wait_for(|&is_paused| !is_paused).await?;
        //
        self.send_event_pure(event_data, force).await
    }
    async fn send_event_pure(
        &mut self,
        event_data: AgentEventData,
        force: bool,
    ) -> anyhow::Result<entity::event::Model> {
        //保存数据库
        let (event, model) = entity_kit::save_event(&self.info, event_data, force).await?;
        self.event_sender.send(event).await?;
        Ok(model)
    }
}

fn to_sse_event<T>(event: &str, data: T) -> sse::Event
where
    T: serde::Serialize,
{
    let payload = json!({
        "event":event,
        "data":data,
    });
    sse::Event::default().json_data(payload).unwrap()
}

impl AgentRuntime {
    pub async fn react_loop(&mut self) -> anyhow::Result<()> {
        loop {
            {
                let mut agent = self.agent.write().await;
                let new_step = entity_kit::incr_agent_step(agent.info.id, agent.info.step)
                    .await
                    .unwrap();
                agent.info.step = new_step;
            }
            self.react().await?;
        }
    }

    async fn react(&mut self) -> anyhow::Result<()> {
        let mut sections = vec![];
        let mut tool_calls = vec![];
        let mut tool_call_ids = vec![];
        let mut last_head_tag = None;
        let mut start_time = 0;

        while let Some(section_event) = self.section_receiver.recv().await {
            match section_event {
                SectionEvent::Start => {
                    send_sse_event(self.agent.clone(), to_sse_event("start", "")).await;
                }
                SectionEvent::Open(tag) => {
                    last_head_tag = Some(tag.clone());
                    start_time = Local::now().timestamp_subsec_millis();
                    let agent = self.agent.read().await;
                    if agent.web_sender.is_some() {
                        send_sse_event(self.agent.clone(), to_sse_event("open", &tag.name)).await;
                    }
                }
                SectionEvent::Chunk(tag_name, chunk) => match tag_name {
                    "reply" | "think" => {
                        let agent = self.agent.clone();
                        send_sse_event(agent.clone(), to_sse_event("chunk", chunk)).await;
                    }
                    _ => {
                        continue;
                    }
                },
                SectionEvent::Close(section) => {
                    //保存数据库
                    let model = {
                        let end_time = Local::now().timestamp_subsec_millis();
                        let mut taken_time = 0;
                        if end_time > start_time {
                            taken_time = end_time - start_time;
                        }
                        let agent = self.agent.read().await;
                        entity_kit::save_section(
                            &agent.info,
                            &section,
                            taken_time,
                            json!({
                                "content":section.content().clone()
                            }),
                        )
                        .await
                        .expect("保存section失败")
                    };
                    //发送到前端
                    let tag = last_head_tag.unwrap();
                    let content = match tag.name {
                        "reply" | "think" => String::new(),
                        _ => (&section).content().to_string(),
                    };
                    let data = json!({
                        "id":model.id,
                        "name":tag.name,
                        "content":content,
                        "meta":model.meta,
                    });
                    send_sse_event(self.agent.clone(), to_sse_event("close", data)).await;
                    //
                    if let Section::ToolCall(code) = &section {
                        tool_call_ids.push(model.id);
                        tool_calls.push(code.to_string());
                    }
                    sections.push(section);
                    last_head_tag = None;
                }
                SectionEvent::End => {
                    //没有调用的话，则等待外部事件触发循环
                    if tool_calls.is_empty() {
                        send_sse_event(self.agent.clone(), to_sse_event("end", "")).await;
                        break;
                    }
                    //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                    self.call_watcher.send(true).unwrap();
                    //批执行
                    let results = self.tool_excutor.batch_call(tool_calls).await;
                    //格式化+关联id
                    let results = tool_call_ids
                        .into_iter()
                        .zip(results)
                        .map(|(id, result)| {
                            let content = match result {
                                Ok(e) => format!("Ok:{}", e),
                                Err(e) => format!("Error:{:?}", e),
                            };
                            ToolCallResultItem{
                                id,content
                            }
                        })
                        .collect();
                    //发送事件
                    {
                        let mut agent = self.agent.write().await;
                        let _ = agent
                            .send_event_pure(AgentEventData::ToolCallResult(results), true)
                            .await
                            .unwrap();
                    }
                    //解锁
                    self.call_watcher.send(false).unwrap();
                    break;
                }
            }
        }
        //保存一个空事件
        let agent = self.agent.read().await;
        entity_kit::save_event(&agent.info, AgentEventData::AgentResponse, false)
            .await
            .expect("保存agent回复失败");
        Ok(())
    }
}

async fn send_sse_event(agent: Arc<RwLock<Agent>>, e: sse::Event) {
    let read = agent.read().await;
    if let Some(sender) = &(&*read).web_sender {
        match sender.send(e) {
            Ok(_) => {}
            Err(e) => {
                //断开
                info!("sse连接断开：{}", e);
                drop(read);
                let mut agent = (agent).write().await;
                agent.web_sender = None;
            }
        };
    }
}
