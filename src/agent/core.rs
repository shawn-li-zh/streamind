use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use axum::response::sse;
use chrono::Local;

use sea_orm::{DbErr, TransactionTrait};
use serde_json::json;
use streamind_core::{
    agent::{AgentContext, AgentStatus},
    event::{
        CallItem, EnvEvent, SectionEvent, ToolCallDetailItem, ToolCallEvent, ToolCallResultItem,
        ToolCallStatus,
    },
    tool::engine::{PreprocessResult, ToolEngine, ToolEngineContext},
};
use tokio::sync::{mpsc, oneshot::Receiver, watch, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::{
    agent::{
        context::stream::{self, MindFlag, StreamMind},
        event::AgentEvent,
        global::{AGENT_POOL, ENGINE_POOL, TOOL_PACKAGE_TOOL},
        kit::{entity_kit, message_kit, prompt_kit},
        sse_event::WebEvent,
    },
    global::DB,
};

#[derive(Debug, Clone)]
pub struct AgentChannel {
    pub event_sender: mpsc::Sender<AgentEvent>,
    pub inject_sender: mpsc::UnboundedSender<(String, bool)>,
    pub web_sender: Option<mpsc::UnboundedSender<sse::Event>>,

    //函数调用
    pub call_watcher: watch::Receiver<bool>,
    //重组条目
    pub reorganize_watcher: watch::Receiver<bool>,
    pub run_watcher: watch::Receiver<bool>,
}

impl AgentChannel {
    pub async fn send_event(&mut self, event: AgentEvent) -> anyhow::Result<()> {
        //等待工具调用结束
        self.call_watcher.wait_for(|&is_paused| !is_paused).await?;
        //
        self.send_event_pure(event).await
    }
    async fn send_event_pure(&mut self, event: AgentEvent) -> anyhow::Result<()> {
        self.event_sender.send(event.clone()).await?;
        //发送到前端
        match &event.data {
            EnvEvent::AgentResponse(_) | EnvEvent::UserRequest(_) | EnvEvent::ToolCallResult(_) => {
                //忽略
            }
            _ => {
                info!("发送事件:{:?}", &event.data);
                let web_event = WebEvent::Env(event.data, event.id);
                self.send_web_event(web_event).await;
            }
        }
        Ok(())
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
            return;
        }
        warn!("断连中...");
    }
}
pub struct Agent {
    pub ctx: Arc<RwLock<AgentContext>>,
    pub channel: AgentChannel,
}

impl Agent {
    pub async fn new(model: entity::agent::Model) -> Arc<RwLock<Agent>> {
        let agent_model = model.model.clone();
        //
        let agent_context = entity_kit::build_agent_context(model).await;
        //通道
        let event = mpsc::channel(100);
        let section = mpsc::channel(100);
        let inject = mpsc::unbounded_channel();
        let call_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        let reorganize_watcher: (watch::Sender<bool>, watch::Receiver<bool>) =
            watch::channel(false);
        let run_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);

        //工具执行器
        let tool_engine = {
            let a = agent_context.read().await;
            ToolEngine::new(agent_context.clone(), a.tool_engine_context.clone())
        };
        //初始化
        let channel = AgentChannel {
            event_sender: event.0,
            inject_sender: inject.0,
            web_sender: None,

            call_watcher: call_watcher.1,
            reorganize_watcher: reorganize_watcher.1,
            run_watcher: run_watcher.1,
        };
        let agent = Agent {
            ctx: agent_context.clone(),
            channel: channel.clone(),
        };

        let agent = Arc::new(RwLock::new(agent));
        //运行输出流
        let a = agent.clone();
        tokio::spawn(async move {
            let engine = {
                let p = ENGINE_POOL.read().await;
                let engine = p.get(&agent_model).unwrap().clone();
                engine
            };
            stream::run_mind(
                a,
                engine,
                inject.1,
                event.1,
                section.0,
                reorganize_watcher.0,
                run_watcher.0,
            )
            .await;
        });
        //运行时
        let mut runtime = AgentRuntime {
            agent: agent.clone(),
            section_receiver: section.1,
            call_watcher: call_watcher.0,
            tool_engine: tool_engine,
        };
        tokio::spawn(async move {
            info!("运行时已启动");
            runtime.react_loop().await.unwrap()
        });

        agent
    }
    pub async fn send_event(
        &mut self,
        event_data: EnvEvent,
        force: bool,
    ) -> anyhow::Result<entity::event::Model> {
        self.send_event_block(event_data, force, true).await
    }

    async fn send_event_block(
        &mut self,
        event_data: EnvEvent,
        force: bool,
        block: bool,
    ) -> anyhow::Result<entity::event::Model> {
        let db = DB.get().unwrap();
        let mut channel = self.channel.clone();
        let ctx = self.ctx.clone();
        let model = db
            .transaction::<_, entity::event::Model, anyhow::Error>(|txn| {
                Box::pin(async move {
                    //保存数据库
                    let (event, model) = {
                        let ctx = ctx.read().await;
                        entity_kit::save_event(txn, &ctx.info, event_data, force).await?
                    };
                    if block {
                        channel.send_event(event).await?;
                    } else {
                        channel.send_event_pure(event).await?;
                    }
                    Ok(model)
                })
            })
            .await?;
        Ok(model)
    }
    pub async fn stop(&mut self, reason: String) -> anyhow::Result<()> {
        self.channel
            .inject_sender
            .send((format!("<interrupt>{}</interrupt>", reason), true))?;
        self.channel.run_watcher.wait_for(|v| !*v).await?;
        Ok(())
    }
}

pub struct AgentRuntime {
    pub agent: Arc<RwLock<Agent>>,
    tool_engine: ToolEngine,
    section_receiver: mpsc::Receiver<SectionEvent>,
    call_watcher: watch::Sender<bool>,
}

impl AgentRuntime {
    pub async fn react_loop(&mut self) -> anyhow::Result<()> {
        let ctx = {
            let agent = self.agent.read().await;
            agent.ctx.clone()
        };
        loop {
            {
                let mut ctx = ctx.write().await;
                ctx.info.status = AgentStatus::Idle;
            }
            if let Err(e) = self.react().await {
                let mut agent = self.agent.write().await;
                agent
                    .send_event(EnvEvent::Error(format!("{}", e)), true)
                    .await
                    .unwrap();
            };
            {
                let mut ctx = ctx.write().await;
                let step = ctx.info.step;
                let _ = entity_kit::incr_agent_step(ctx.info.id, step)
                    .await
                    .unwrap();
                ctx.info.step += 1;
                info!("step+1");
            }
        }
    }

    async fn react(&mut self) -> anyhow::Result<()> {
        let mut tool_calls: Vec<String> = vec![];
        let mut resp_event_id = 0;
        //
        let ctx = {
            let agent = self.agent.read().await;
            agent.ctx.clone()
        };
        let mut sections = vec![];
        while let Some(section_event) = self.section_receiver.recv().await {
            match &section_event {
                SectionEvent::Start => {
                    {
                        let mut ctx = ctx.write().await;
                        ctx.info.status = AgentStatus::Running;
                    }
                    {
                        //保存一个空事件
                        let agent = self.agent.read().await;
                        let ctx = agent.ctx.read().await;
                        let (event, model) = entity_kit::save_event(
                            DB.get().unwrap(),
                            &ctx.info,
                            EnvEvent::AgentResponse(vec![]),
                            false,
                        )
                        .await
                        .expect("保存agent回复失败");
                        resp_event_id = model.id;
                    };
                    let web_event = WebEvent::Section(section_event.clone(), resp_event_id);
                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::Open(tag) => {
                    let web_event = WebEvent::Section(section_event.clone(), resp_event_id);
                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::Chunk(tag, _) => match tag.name {
                    "reply" | "think" => {
                        let web_event = WebEvent::Section(section_event.clone(), resp_event_id);
                        send_web_event(self.agent.clone(), web_event).await;
                    }
                    _ => {
                        continue;
                    }
                },
                SectionEvent::Close(section) => {
                    sections.push(section.clone());
                    //发送到前端
                    let web_event = WebEvent::Section(section_event.clone(), resp_event_id);
                    send_web_event(self.agent.clone(), web_event).await;
                    info!("SECTION:{}", &section.tag_name);
                    //
                    if &section.tag_name == "tool_call" {
                        tool_calls.push(section.content.clone());
                        continue;
                    }
                }
                SectionEvent::End => {
                    let web_event = WebEvent::Section(section_event.clone(), resp_event_id);
                    send_web_event(self.agent.clone(), web_event).await;
                    //没有调用的话，则等待外部事件触发循环
                    if tool_calls.is_empty() {
                        break;
                    }
                    //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                    self.call_watcher.send(true).unwrap();
                    if let Err(e) = call(tool_calls, self.agent.clone(), &self.tool_engine).await {
                        error!("{}", &e);
                        //发送事件处理
                        let mut agent = self.agent.write().await;
                        agent
                            .send_event_block(EnvEvent::Error(e.to_string()), true, false)
                            .await?;
                    }
                    //解锁
                    self.call_watcher.clone().send(false).unwrap();
                    break;
                }
            }
        }
        entity_kit::update_agent_response(resp_event_id, sections).await?;
        info!("step end");

        Ok(())
    }
}

async fn call(
    codes: Vec<String>,
    agent: Arc<RwLock<Agent>>,
    tool_engine: &ToolEngine,
) -> anyhow::Result<()> {
    // let call_ids = tool_call_ids.clone();
    let tool_calls = match tool_engine.batch_preprocess(codes) {
        PreprocessResult::Valid(tool_calls) => tool_calls,
        PreprocessResult::Invalid(items) => {
            let items: Vec<String> = items.iter().map(|(id, e)| format!("{e}")).collect();
            return Err(anyhow::anyhow!(items.join("\n")));
        }
    };

    // //保存一个空响应事件
    let mut bind_event = {
        let agent = agent.read().await;
        let ctx = agent.ctx.read().await;
        let (event, model) = entity_kit::save_event(
            DB.get().unwrap(),
            &ctx.info,
            EnvEvent::ToolCallResult(vec![]),
            false,
        )
        .await
        .expect("创建工具调用结果事件失败");
        event
    };
    let bind_event_id = bind_event.id;
    let mut details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
    let mut items = vec![];
    let mut call_ids = vec![];
    for call in &tool_calls {
        call_ids.push(call.id);
        //
        let item = CallItem {
            id: call.id,
            package: call.package.name,
            tool: call.tool.name,
        };
        items.push(item);
        //
        let detail = ToolCallDetailItem {
            id: call.id,
            package: call.package.name.to_string(),
            tool: call.tool.name.to_string(),
            content: "".to_string(),
            status: ToolCallStatus::Undo,
            time: 0,
        };
        details.insert(detail.id, detail);
    }
    let (sender, mut receiver) = mpsc::channel::<ToolCallEvent>(tool_calls.len());
    //批执行
    let agent_ = agent.clone();
    tokio::spawn(async move {
        let agent = agent_;
        //异步处理函数调用结果
        while let Some(event) = receiver.recv().await {
            send_web_event(
                agent.clone(),
                WebEvent::ToolCall(event.clone(), bind_event_id),
            )
            .await;
            let item = match event {
                ToolCallEvent::Success(item) => item,
                ToolCallEvent::Failure(item) => item,
                _ => {
                    continue;
                }
            };
            let detial = details.get_mut(&item.id).unwrap();
            detial.content = item.content;
            detial.status = item.status;
            detial.time = item.time.subsec_millis();
        }
        //排序
        let details: Vec<ToolCallDetailItem> = call_ids
            .iter()
            .map(|id| details.remove(id).unwrap())
            .collect();
        entity_kit::update_tool_call_result(bind_event_id, details.clone())
            .await
            .unwrap();
        //发送完整事件
        bind_event.data = EnvEvent::ToolCallResult(details);
        let mut agent = agent.write().await;
        agent.channel.send_event_pure(bind_event).await.unwrap();
    });
    //通知前端 工具调用开始执行
    send_web_event(
        agent.clone(),
        WebEvent::ToolCall(ToolCallEvent::Start(items), bind_event_id),
    )
    .await;
    tool_engine.batch_call(tool_calls, sender).await?;
    //结束
    send_web_event(
        agent.clone(),
        WebEvent::ToolCall(ToolCallEvent::End, bind_event_id),
    )
    .await;
    Ok(())
}

async fn send_web_event(agent: Arc<RwLock<Agent>>, e: WebEvent) {
    let mut agent = agent.write().await;
    agent.channel.send_web_event(e).await
}
