use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_openai::types::ChatCompletionRequestSystemMessageArgs;
use axum::response::sse;
use chrono::Local;

use serde_json::json;
use tokio::sync::{mpsc, oneshot::Receiver, watch, Mutex, RwLock};
use tracing::info;

use crate::{
    agent::{
        event::{AgentEvent, AgentEventData, ToolCallResultItem},
        global::{AGENT_POOL, ENGINE_POOL},
        kit::{entity_kit, message_kit, prompt_kit},
        section::{Section, SectionEvent},
        sse_event::WebEvent,
        stream::MindStream,
    },
    tool::{
        core::ToolCallError,
        executor::{ToolCallEvent, ToolExcutor},
    },
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
        self.event_sender.send(event.clone()).await?;
        //发送到前端
        match &event.data {
            AgentEventData::AgentResponse
            | AgentEventData::UserInput(_)
            | AgentEventData::ToolCallResult(_) => {
                //忽略
            }
            _ => {
                let web_event = WebEvent::Agent(event);
                self.send_web_event(web_event).await;
            }
        }

        Ok(model)
    }

    async fn send_web_event(&mut self, e: WebEvent) {
        if let Some(sender) = &self.web_sender {
            match sender.send(e.to_sse_event()) {
                Ok(_) => {}
                Err(e) => {
                    //断开
                    info!("sse连接断开：{}", e);
                    self.web_sender = None;
                }
            };
        }
    }
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
            if let Err(e) = self.react().await {
                let mut agent = self.agent.write().await;
                agent
                    .send_event(AgentEventData::Error(format!("{}", e)), true)
                    .await
                    .unwrap();
            };
        }
    }

    async fn react(&mut self) -> anyhow::Result<()> {
        let mut tool_calls = vec![];
        let mut start_time = 0;

        while let Some(section_event) = self.section_receiver.recv().await {
            let web_event = WebEvent::Section(section_event.clone(), 0);
            match section_event {
                SectionEvent::Start => {
                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::Open(tag) => {
                    start_time = Local::now().timestamp_subsec_millis();

                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::Chunk(tag, _) => match tag.name {
                    "reply" | "think" => {
                        send_web_event(self.agent.clone(), web_event).await;
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
                                "content":section.content.clone()
                            }),
                        )
                        .await
                        .expect("保存section失败")
                    };
                    //发送到前端
                    let web_event =
                        WebEvent::Section(SectionEvent::Close(section.clone()), model.id);
                    send_web_event(self.agent.clone(), web_event).await;
                    //
                    if section.tag.name == "tool_call" {
                        tool_calls.push((model.id, section.content));
                        continue;
                    }
                }
                SectionEvent::End => {
                    //没有调用的话，则等待外部事件触发循环
                    if tool_calls.is_empty() {
                        send_web_event(self.agent.clone(), web_event).await;
                        break;
                    }

                    //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                    self.call_watcher.send(true).unwrap();
                    //批执行
                    let (sender, mut receiver) = mpsc::channel::<ToolCallEvent>(tool_calls.len());
                    let agent = self.agent.clone();
                    let call_watcher = self.call_watcher.clone();
                    let tool_call_ids: Vec<i64> = tool_calls.iter().map(|(id, _)| *id).collect();
                    let call_ids = tool_call_ids.clone();
                    tokio::spawn(async move {
                        //异步处理函数调用结果
                        let mut map: HashMap<i64, ToolCallResultItem> = HashMap::new();
                        while let Some(event) = receiver.recv().await {
                            let (id, content) = match &event {
                                ToolCallEvent::Active(_, _) => {
                                    send_web_event(agent.clone(), WebEvent::ToolCall(event)).await;
                                    continue;
                                }
                                ToolCallEvent::Success(id, c) => (*id, format!("Ok:{}", c)),
                                ToolCallEvent::Failure(id, e) => (*id, format!("Error:{}", e)),
                                _ => {
                                    continue;
                                }
                            };
                            map.insert(id, ToolCallResultItem { id, content });
                            send_web_event(agent.clone(), WebEvent::ToolCall(event)).await;
                        }
                        //结束
                        send_web_event(agent.clone(), WebEvent::ToolCall(ToolCallEvent::End)).await;
                        //发送完整事件
                        {
                            //排序
                            let results = call_ids
                                .iter()
                                .map(|(id)| map.remove(id).unwrap())
                                .collect();
                            let mut agent = agent.write().await;
                            let _ = agent
                                .send_event_pure(AgentEventData::ToolCallResult(results), true)
                                .await
                                .unwrap();
                        }
                        //解锁
                        call_watcher.clone().send(false).unwrap();
                    });
                    //通知前端 工具调用开始执行
                    let agent = self.agent.clone();

                    send_web_event(
                        agent.clone(),
                        WebEvent::ToolCall(ToolCallEvent::Start(tool_call_ids)),
                    )
                    .await;
                    self.tool_excutor.batch_call(tool_calls, sender).await?;
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

async fn send_web_event(agent: Arc<RwLock<Agent>>, e: WebEvent) {
    let mut agent = agent.write().await;
    agent.send_web_event(e).await
}
