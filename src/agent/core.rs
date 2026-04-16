use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicI64, AtomicU32, Ordering},
        Arc,
    },
};

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
        event::{AgentEvent, AgentEventData},
        global::{AGENT_POOL, ENGINE_POOL},
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
    step: Arc<AtomicI64>,
    pub web_sender: Option<mpsc::UnboundedSender<sse::Event>>,
}

pub struct AgentRuntime {
    pub agent: Arc<RwLock<Agent>>,
    tool_excutor: ToolExcutor,
    section_receiver: mpsc::Receiver<SectionEvent>,
    call_watcher: watch::Sender<bool>,
    step: Arc<AtomicI64>,
}

impl Agent {
    pub async fn new(info: entity::agent::Model) -> Arc<RwLock<Agent>> {
        //通道
        let event = mpsc::channel(100);
        let section = mpsc::channel(100);
        let inject = mpsc::unbounded_channel();
        let call_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        let run_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);

        //流
        let p = ENGINE_POOL.read().await;
        let engine = p.get("default").unwrap().clone();
        let system_prompt = build_system_prompt(&info).await;
        let mut mind_stream = MindStream::new(
            engine,
            system_prompt,
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
        let step = Arc::new(AtomicI64::new(info.step));
        let agent = Agent {
            info,
            event_sender: event.0,
            call_watcher: call_watcher.1,
            run_watcher: run_watcher.1,
            inject_sender: inject.0,
            web_sender: None,
            step: step.clone(),
        };
        let agent = Arc::new(RwLock::new(agent));
        let mut runtime = AgentRuntime {
            agent: agent.clone(),
            section_receiver: section.1,
            call_watcher: call_watcher.0,
            tool_excutor,
            step,
        };
        tokio::spawn(async move {
            loop {
                runtime.react().await.unwrap();
            }
        });

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
    ) -> anyhow::Result<()> {
        self.call_watcher.wait_for(|&is_paused| !is_paused).await?;
        //
        self.send_event_pure(event_data, force).await
    }
    async fn send_event_pure(
        &mut self,
        event_data: AgentEventData,
        force: bool,
    ) -> anyhow::Result<()> {
        //保存数据库
        let event = AgentEvent {
            id: 0,
            agent_id: self.info.id,
            data: event_data,
            force,
            time: Local::now(),
        };
        let event_model = entity::event::ActiveModel {
            agent_id: Set(event.agent_id),
            state_version: Set(self.info.state_version),
            step: Set(self.step.load(Ordering::Relaxed)),
            event_type: Set(event.data.name().to_string()),
            force: Set(event.force),
            time: Set(event.time),
            content: Set(event.to_message()),
            ..Default::default()
        };
        event_model.insert(DB.get().unwrap()).await?;
        self.event_sender.send(event).await?;
        Ok(())
    }
}

async fn build_system_prompt(info: &entity::agent::Model) -> String {
    let items = entity::item::Entity::find()
        .filter(entity::item::Column::AgentId.eq(info.id))
        .all(DB.get().unwrap())
        .await
        .unwrap();
    let mut memory = String::new();
    let mut focus = String::new();
    let mut goal = String::new();
    for item in items {
        let part = match item.group.as_str() {
            "memory" => &mut memory,
            "focus" => &mut focus,
            "goal" => &mut goal,
            _ => {
                continue;
            }
        };
        part.push_str("- ");
        part.push_str(&item.name);
        part.push_str(": ");
        part.push_str(&item.content);
        part.push_str("\n");
    }

    let generate_rule = PROMPT_CACHE.get("common/generate_rule").unwrap_or_default();
    let tool_call_rule = PROMPT_CACHE
        .get("common/tool_call_rule")
        .unwrap_or_default();

    format!(
        r#"
    # 基础信息:
    - 编号:{}
    - 名称:{}

    # 生成格式
    {}

    # 工具调用规则
    {}

    # 记忆
    {}

    # 关注
    {}

    # 目标
    {}
    "#,
        info.id, info.name, generate_rule, tool_call_rule, memory, focus, goal
    )
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
            self.step.fetch_add(1, Ordering::Relaxed);
            self.react().await?;
        }
    }

    async fn react(&mut self) -> anyhow::Result<()> {
        let mut sections = vec![];
        let mut tool_calls = vec![];
        let mut last_head_tag = None;
        while let Some(section_event) = self.section_receiver.recv().await {
            match section_event {
                SectionEvent::Start => {
                    send_sse_event(self.agent.clone(), to_sse_event("start", "")).await;
                }
                SectionEvent::Open(tag) => {
                    last_head_tag = Some(tag.clone());
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
                    //保存数据库+发送到前端
                    let tag = last_head_tag.unwrap();
                    match tag.name {
                        "reply" | "think" => {
                            send_sse_event(self.agent.clone(), to_sse_event("close", tag.name))
                                .await;
                        }
                        _ => {
                            let c: String = (&section).into();
                            send_sse_event(self.agent.clone(), to_sse_event("close", c)).await;
                        }
                    }
                    if let Section::ToolCall(code) = &section {
                        tool_calls.push(code.to_string());
                    }
                    sections.push(section);
                    last_head_tag = None;
                }
                SectionEvent::End => {
                    info!("end===");
                    send_sse_event(self.agent.clone(), to_sse_event("end", "")).await;
                    //如果有函数调用，则执行函数调用，然后发送事件继续循环，
                    //没有的话，则等待外部事件触发循环
                    if tool_calls.is_empty() {
                        break;
                    }
                    //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                    self.call_watcher.send(true).unwrap();
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
                    {
                        let mut agent = self.agent.write().await;
                        let _ = agent
                            .send_event_pure(AgentEventData::ToolCallResult(results), true)
                            .await
                            .unwrap();
                    }
                    self.call_watcher.send(false).unwrap();
                    break;
                }
            }
            continue;
        }
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
