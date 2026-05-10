use std::{
    backtrace::Backtrace,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use axum::response::sse;
use chrono::Local;

use dashmap::DashMap;
use entity::event::EnvEventType;
use entity_service::DB;
// use sea_orm::{DbErr, TransactionTrait};
use kernel::{
    agent::{AgentContext, AgentEvent, AgentInfo, AgentStatus, PermissionLife},
    event::{
        CallItem, EnvEvent, SectionEvent, ToolCallDetailItem, ToolCallEvent, ToolCallResultItem,
        ToolCallStatus,
    },
    render::RenderContext,
    section::Section,
    tool::{
        call::ToolCall,
        core::ToolReturn,
        engine::{PreprocessResult, ToolEngine, ToolEngineContext},
    },
};
use serde::de;
use serde_json::json;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot::Receiver, watch};
use tracing::{debug, error, info, warn};

use crate::{
    context::stream,
    global::{AGENT_CTX_POOL, AGENT_POOL, ENGINE_POOL, TOOL_PACKAGE_TOOL},
    sse_event::WebEvent,
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
    pub ctx: Arc<AgentContext>,
    pub channel: AgentChannel,
}

impl Agent {
    pub async fn new(agent: entity::agent::Model) -> Arc<RwLock<Agent>> {
        let agent_id = agent.id;
        let agent_name = agent.name.clone();
        let model_id = agent.model_id;
        //
        let agent_context = build_agent_context(agent).await;
        //通道
        let event = mpsc::channel(100);
        let section = mpsc::channel(100);
        let inject: (
            mpsc::UnboundedSender<(String, bool)>,
            mpsc::UnboundedReceiver<(String, bool)>,
        ) = mpsc::unbounded_channel();
        let call_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        let reorganize_watcher: (watch::Sender<bool>, watch::Receiver<bool>) =
            watch::channel(false);
        let run_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);

        //工具执行器
        let tool_engine = {
            ToolEngine::new(
                agent_context.clone(),
                agent_context.tool_engine_context.clone(),
            )
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
                let engine = p.get(&model_id).unwrap().clone();
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
            agent_id,
            section_receiver: section.1,
            call_watcher: call_watcher.0,
            tool_engine: tool_engine,
        };
        tokio::spawn(async move {
            entity_service::agent::startup(DB.get().unwrap(), agent_id)
                .await
                .unwrap();
            info!("智能体【{}】运行时已启动", agent_name);
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
        let mut channel = self.channel.clone();
        let ctx = self.ctx.clone();
        let model = entity_service::transaction::<_, entity::event::Model>(|txn| {
            Box::pin(async move {
                //保存数据库
                let (event, model) = {
                    let ctx = ctx.clone();
                    entity_service::event::save(txn, &ctx.info, event_data, force).await?
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
    agent_id: u32,
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
            // {
            //     let mut ctx = ctx.clone();
            //     ctx.info.status = AgentStatus::Idle;
            // }
            if let Err(e) = self.react().await {
                let mut agent = self.agent.write().await;
                agent
                    .send_event(EnvEvent::Error(format!("{}", e)), true)
                    .await
                    .unwrap();
            };
        }
    }

    async fn react(&mut self) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        //
        let mut tool_calls: Vec<ToolCall> = vec![];
        let mut tool_call_details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
        let mut response_event_id = 0;
        let mut reaction_event_id = 0;
        //
        let ctx = {
            let agent = self.agent.read().await;
            agent.ctx.clone()
        };
        let mut response_sections = vec![];
        let mut reaction_sections = vec![];
        while let Some(section_event) = self.section_receiver.recv().await {
            // let sec_event = section_event.clone();
            match section_event {
                SectionEvent::Start(_, _) => {
                    // {
                    //     let mut ctx = ctx.write().await;
                    //     ctx.info.status = AgentStatus::Running;
                    // }
                    {
                        //
                        let (event, model) = entity_service::event::save(
                            db,
                            &ctx.info,
                            EnvEvent::AgentReaction(vec![]),
                            false,
                        )
                        .await
                        .expect("保存agent反应失败");
                        reaction_event_id = model.id;
                        //保存一个空事件
                        let (event, model) = entity_service::event::save(
                            db,
                            &ctx.info,
                            EnvEvent::AgentResponse(vec![]),
                            false,
                        )
                        .await
                        .expect("保存agent回复失败");
                        response_event_id = model.id;
                    };
                    let web_event = WebEvent::Section(
                        SectionEvent::Start(reaction_event_id, response_event_id),
                        reaction_event_id,
                    );
                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::Open(tag) => {
                    let event_id = match tag.name {
                        "reply" | "render" => response_event_id,
                        _ => reaction_event_id,
                    };
                    let web_event = WebEvent::Section(section_event.clone(), event_id);
                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::Chunk(tag, _) => {
                    let event_id = match tag.name {
                        "reply" | "render" => response_event_id,
                        _ => reaction_event_id,
                    };
                    match tag.name {
                        "reply" | "think" => {
                            let web_event = WebEvent::Section(section_event.clone(), event_id);
                            send_web_event(self.agent.clone(), web_event).await;
                        }
                        _ => {
                            continue;
                        }
                    }
                }
                SectionEvent::Close(mut section) => {
                    let (event_id, sections) = match section.tag_name.as_str() {
                        "reply" | "render" => (response_event_id, &mut response_sections),
                        _ => (reaction_event_id, &mut reaction_sections),
                    };
                    //保存section
                    {
                        let model = entity_service::section::save(
                            db,
                            self.agent_id,
                            event_id,
                            section.clone(),
                            serde_json::Value::Null,
                        )
                        .await?;
                        section.id = model.id;
                        sections.push(section.clone());
                    };
                    let payload = match section.tag_name.as_str() {
                        "render" => {
                            let section_content = section.content.trim();
                            let ctx: RenderContext = serde_json::from_str(section_content)?;
                            serde_json::to_value(ctx)?
                        }
                        "tool_call" => {
                            let call = self
                                .tool_engine
                                .preprocess(section.id, section.content.trim())?;
                            let detail = ToolCallDetailItem {
                                id: call.id,
                                package: call.package.name.to_string(),
                                tool: call.tool.name.to_string(),
                                result: ToolReturn::new(),
                                status: ToolCallStatus::Undo,
                                args: call.args.clone(),
                                elapsed: "".to_string(),
                            };
                            tool_call_details.insert(call.id, detail.clone());
                            tool_calls.push(call);
                            json!(&detail)
                        }
                        _ => serde_json::Value::Null,
                    };
                    entity_service::section::update_payload(db, section.id, payload).await?;
                    //发送到前端
                    info!("SECTION:{}", &section.tag_name);
                    let web_event = WebEvent::Section(SectionEvent::Close(section), event_id);
                    send_web_event(self.agent.clone(), web_event).await;
                }
                SectionEvent::End(_, _) => {
                    let web_event = WebEvent::Section(
                        SectionEvent::End(reaction_event_id, response_event_id),
                        response_event_id,
                    );
                    send_web_event(self.agent.clone(), web_event).await;
                    //没有调用的话，则等待外部事件触发循环
                    if tool_calls.is_empty() {
                        break;
                    }
                    //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                    self.call_watcher.send(true).unwrap();
                    if let Err(e) = tool_call(
                        tool_calls,
                        tool_call_details,
                        self.agent.clone(),
                        &self.tool_engine,
                    )
                    .await
                    {
                        let bt = Backtrace::capture();
                        eprintln!("{:?}", bt);
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
        entity_service::event::update_agent_response(
            db,
            reaction_event_id,
            EnvEventType::AgentReaction,
            reaction_sections,
        )
        .await?;
        entity_service::event::update_agent_response(
            db,
            response_event_id,
            EnvEventType::AgentResponse,
            response_sections,
        )
        .await?;
        info!("step end");

        Ok(())
    }
}

async fn tool_call(
    tool_calls: Vec<ToolCall>,
    mut tool_call_details: HashMap<i64, ToolCallDetailItem>,
    agent: Arc<RwLock<Agent>>,
    tool_engine: &ToolEngine,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    // //保存一个空响应事件
    let mut bind_event = {
        let agent = agent.read().await;
        let ctx = agent.ctx.clone();
        let (event, model) =
            entity_service::event::save(db, &ctx.info, EnvEvent::ToolCallResult(vec![]), false)
                .await
                .expect("创建工具调用结果事件失败");
        event
    };
    let bind_event_id = bind_event.id;
    let mut items = vec![];
    let mut call_ids = vec![];
    for call in &tool_calls {
        call_ids.push(call.id);
        //
        let item = CallItem {
            id: call.id,
            package: call.package.name,
            tool: call.tool.name,
            args: call.args.clone(),
        };
        items.push(item);
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
            let detial = tool_call_details.get_mut(&item.id).unwrap();
            detial.result = item.result;
            detial.status = item.status;
            detial.elapsed = humantime::format_duration(item.time).to_string();
        }
        //排序
        let details: Vec<ToolCallDetailItem> = call_ids
            .iter()
            .map(|id| tool_call_details.remove(id).unwrap())
            .collect();
        entity_service::event::update_tool_call_result(db, bind_event_id, details.clone())
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

pub async fn build_agent_context(agent: entity::agent::Model) -> Arc<AgentContext> {
    let pool = ENGINE_POOL.read().await;
    let engine = pool.get(&agent.model_id).expect("模型不存在");

    let agent_info = AgentInfo {
        id: agent.id,
        code: agent.code,
        model_id: agent.model_id,
        parent_id: agent.parent_id,
        state_id: agent.state_id,
        // state_pos: model.state_pos,
        name: agent.name,
        model: engine.model.name.clone(),
        workspace: PathBuf::from(&agent.workspace),
        avatar: agent.avatar,
        status: AgentStatus::Idle,
    };
    //初始化工作空间
    let space = std::env::current_dir().unwrap().join(&agent.workspace);
    if !space.exists() {
        std::fs::create_dir_all(space).unwrap();
    }
    //tool engine context
    let tool_engine_context = {
        let mut tool_pkgs = HashSet::new();
        for pkg_name in &agent.tool_packages.0 {
            if pkg_name == "*" {
                tool_pkgs.extend(TOOL_PACKAGE_TOOL.keys());
                break;
            }
            tool_pkgs.insert(
                TOOL_PACKAGE_TOOL
                    .get(pkg_name.as_str())
                    .expect("包不存在")
                    .name,
            );
        }
        ToolEngineContext::build(&agent_info, tool_pkgs, &*TOOL_PACKAGE_TOOL)
    };
    //获取权限
    let permissions = {
        let permissions = entity_service::agent_permission::list(agent.id)
            .await
            .unwrap();
        let map = DashMap::new();
        for item in permissions {
            let life: PermissionLife = serde_json::from_value(item.life).unwrap();

            let p = kernel::agent::AgentPermission {
                code: item.code,
                sub_code: item.sub_code,
                life: life,
                use_times: item.use_times,
                create_time: item.create_time,
            };
            map.insert(p.code.clone(), p);
        }
        map
    };
    let agent_context = AgentContext {
        info: agent_info,
        tool_engine_context: Arc::new(tool_engine_context),
        permissions,
    };
    //
    let id = agent_context.info.id;

    let agent_context = Arc::new(agent_context);
    //
    let mut p = AGENT_CTX_POOL.write().await;
    p.insert(id, agent_context.clone());
    agent_context
}
