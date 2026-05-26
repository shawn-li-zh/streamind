use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
};

use anyhow::Ok;
use async_trait::async_trait;
use axum::response::sse;

use entity_service::DB;

use futures::FutureExt as _;
use kernel::{
    agent::{
        context::{AgentContext, AgentStatus},
        event::EnvEvent,
        port::AgentOutlet,
        AgentIdentity, AgentOutfit,
    },
    message::Information,
    mind::{
        context::EventContext as _,
        port::{CommandEvent, MindOutlet},
    },
    tool::engine::ToolEngineContext,
};
use parking_lot::RwLock;
use reflection::{engine::engine::RuleEngine, operator::EventElement};
use runtime::agent::{
    authority::{AgentAuthorityContext, AgentPermission, PermissionLife},
    event::{AppEnvEvent, AppEnvEventData},
    event_context::StackContext,
    instance::{build_agent_instance, AgentInstance, EventLoop},
    reflect::{ReflectData, ReflectEvent},
    scheduler::{AgentSchedulerContext, AgentTimer},
};

use tokio::sync::{
    mpsc::{self, Receiver},
    watch, Mutex,
};
use tool_pkg::{reflect::ReflectInjected, timer::TimerInjected};
use tracing::{error, info, warn};

use crate::{
    event::convert,
    event_context::load_stack,
    global::{AGENT_POOL, EMBEDDING_CLIENT, ENGINE_POOL, TOOL_PACKAGE_TOOL},
    logger::{self, AgentLogger},
    plugin::{
        authority::AgentAuthority,
        heartbeat::{Heartbeat, HeartbeatRunner},
        reflect::{self, load_rule_groups},
        scheduler::{AgentScheduler, AgentSchedulerRunner},
    },
    runtime::AgentRuntime,
    sse_event::WebEvent,
};
pub struct AppAgent {
    pub(crate) instance: Arc<AgentInstance>,
    pub(crate) heartbeat: Arc<Heartbeat>,
    pub(crate) web_sender: Arc<WebSender>,
    gen_watcher: Mutex<watch::Receiver<bool>>,
    logger: Arc<AgentLogger>,
    pub(crate) reflect_engine: Arc<RuleEngine<ReflectData>>,
}
impl AppAgent {
    pub async fn new(model: entity::agent::Model) -> (Arc<Self>, AgentRuntime) {
        let (instance, event_loop, outlet, timer_rx) = build_instance(&model).await;
        let instance = Arc::new(instance);
        let heartbeat = Arc::new(Heartbeat::new());
        let gen = watch::channel(false);
        let reflect_engine: RuleEngine<ReflectData> =
            reflect::load_reflection_engine(&model).await.unwrap();
        //日志
        let logger = AgentLogger::new(&instance.identity().workspace)
            .await
            .unwrap();
        let logger = Arc::new(logger);
        let agent = AppAgent {
            instance: instance.clone(),
            heartbeat: heartbeat.clone(),
            web_sender: Arc::new(WebSender {
                inner: RwLock::new(None),
            }),
            gen_watcher: Mutex::new(gen.1),
            logger: logger.clone(),
            reflect_engine: Arc::new(reflect_engine),
        };

        let agent = Arc::new(agent);

        //
        let heartbeat_runner = HeartbeatRunner::new(heartbeat, agent.clone());
        let runner = AgentSchedulerRunner::new(agent.clone(), timer_rx);
        let runtime = AgentRuntime::new(
            agent.clone(),
            event_loop,
            outlet,
            heartbeat_runner,
            runner,
            gen.0,
        );

        (agent, runtime)
    }
}
#[async_trait]
impl AgentOutfit<AppEnvEvent, StackContext<AppEnvEvent>> for AppAgent {
    async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()> {
        //发送给代理
        self.instance.send_cmd(cmd).await?;
        Ok(())
    }

    fn event_context(&self) -> Arc<StackContext<AppEnvEvent>> {
        self.instance.event_context()
    }
    fn context(&self) -> Arc<AgentContext> {
        self.instance.context()
    }
    fn identity(&self) -> Arc<AgentIdentity> {
        self.instance.identity()
    }
    async fn shutdown(&self) -> anyhow::Result<()> {
        let context = self.context();
        if let Some(tool_engine_context) = context.get::<ToolEngineContext>() {
            tool_engine_context.shutdown().await?;
        }
        Ok(())
    }
}
impl AppAgent {
    pub async fn send_event(&self, event: (AppEnvEventData, bool)) -> anyhow::Result<i64> {
        //保存数据库
        let env_event = self.save_event(event).await?;
        let id = env_event.id;
        //发送给web
        match &env_event.data {
            AppEnvEventData::AgentResponse(_) => {
                //响应事件不参与agent处理
                return Ok(id);
            }
            AppEnvEventData::ToolResponse(_)
            | AppEnvEventData::Interaction(_)
            | AppEnvEventData::Reflection(_)
            | AppEnvEventData::Heartbeat => {
                //忽略
            }
            _ => {
                self.web_sender
                    .send(WebEvent::Env(env_event.data.clone(), id));
            }
        }
        info!("AgentEvent: {:?}", &env_event.data);
        //反射
        let reflection = self.event_reflect(&env_event).await?;
        //发送给代理
        if env_event.force {
            self.instance.send_cmd(CommandEvent::Canceled).await?;
        }
        //等待生成完成
        let mut gen = self.gen_watcher.lock().await;
        gen.wait_for(|v| !*v).await?;
        //发送事件给上下文
        self.inject_event_to_context(env_event, reflection).await?;
        self.instance.send_cmd(CommandEvent::Generate).await?;
        self.heartbeat.disable();
        Ok(id)
    }

    pub async fn save_event(&self, event: (AppEnvEventData, bool)) -> anyhow::Result<AppEnvEvent> {
        let (data, force) = event;
        //保存数据库
        let db = DB.get().unwrap();
        //保存数据库
        let model = {
            let identity = self.identity();
            let state_id = {
                if let Some(top_node) = self.event_context().top_node() {
                    top_node.id
                } else {
                    anyhow::bail!("没有状态节点");
                }
            };
            let ty = convert::to_type_enum(&data);
            let value = convert::to_json(&data);
            entity_service::event::save(db, identity.id(), state_id, ty, value, force, data.size())
                .await?
        };
        let env_event = AppEnvEvent {
            id: model.id,
            data: data,
            force,
            time: model.time,
            size: model.size as _,
        };
        return Ok(env_event);
    }
    pub(crate) async fn inject_event_to_context(
        &self,
        env_event: AppEnvEvent,
        reflection: Option<ReflectEvent>,
    ) -> anyhow::Result<()> {
        //
        self.logger.log_event(&env_event).await?;
        let event_context = self.instance.event_context();
        event_context.push_event(env_event);
        //按顺序注入反射事件
        if let Some(reflection) = reflection {
            let reflection = AppEnvEventData::Reflection(reflection);
            let env_event = self.save_event((reflection, false)).await?;
            self.logger.log_event(&env_event).await?;
            event_context.push_event(env_event);
        }
        Ok(())
    }
    //事件反射
    pub(crate) async fn event_reflect(
        &self,
        event: &AppEnvEvent,
    ) -> anyhow::Result<Option<ReflectEvent>> {
        let mut source_event_id = event.id;
        let mut level = 1;
        match &event.data {
            AppEnvEventData::Reflection(ev) => {
                level = ev.level + 1;
                if level > 2 {
                    //最多反射两层
                    return Ok(None);
                }
                source_event_id = ev.source_event_id;
            }
            _ => {}
        }
        //
        let (text, _paths) = event.to_message().to_tuple();
        let client = EMBEDDING_CLIENT.get().unwrap();
        let embedding = client
            .union_embed(vec![Information::Text(text.clone())])
            .await?;

        let element = EventElement {
            event_type: event.data.code(),
            time: event.time.format("%Y-%m-%d %H:%M:%S").to_string(),
            force: event.force,
            context: text,
            embedding,
        };

        let datas = self.reflect_engine.trigger(element).await?;
        if datas.is_empty() {
            return Ok(None);
        }
        let event = ReflectEvent {
            source_event_id,
            level,
            datas,
        };
        Ok(Some(event))
    }
}

pub struct WebSender {
    inner: RwLock<Option<mpsc::UnboundedSender<sse::Event>>>,
}
impl WebSender {
    pub fn send(&self, event: WebEvent) {
        let web_sender = self.inner.read();
        if let Some(sender) = &*web_sender {
            if let Err(e) = sender.send(event.to_sse_event()) {
                //断开
                info!("sse连接断开：{}", e);
                drop(web_sender);
                let mut web_sender = self.inner.write();
                *web_sender = None;
            }
            return;
        }
        // warn!("断连中...");
    }
    pub fn set_sender(&self, sender: mpsc::UnboundedSender<sse::Event>) {
        let mut web_sender = self.inner.write();
        *web_sender = Some(sender);
    }
}

pub async fn build_instance(
    agent: &entity::agent::Model,
) -> (
    AgentInstance,
    EventLoop,
    MindOutlet,
    Receiver<(Arc<AgentTimer>, bool)>,
) {
    let model_id = agent.model_id;
    //
    let identity = build_agent_identity(&agent);
    let context = build_agent_context(&identity).await;
    //上下文流处理器
    let client = {
        let p = ENGINE_POOL.read();
        let client = p.get(&model_id).unwrap().clone();
        client
    };
    let stack = load_stack(identity.as_ref(), client.clone(), &context)
        .await
        .unwrap();
    //定时器
    let ctx = context.get::<AgentSchedulerContext>().unwrap();
    let (scheduler, timer_rx) = AgentScheduler::new(identity.clone(), ctx);
    let scheduler: Arc<AgentScheduler> = Arc::new(scheduler);
    //权限
    let ctx = context.get::<AgentAuthorityContext>().unwrap();
    let authority: Arc<AgentAuthority> = Arc::new(AgentAuthority {
        identity: identity.clone(),
        context: ctx,
    });
    //初始化工具上下文
    if let Some(ctx) = context.get::<ToolEngineContext>() {
        ctx.init(context.clone());
    }
    let (ins, event_loop, outlet) =
        build_agent_instance(identity, context, Arc::new(stack), scheduler, authority);

    (ins, event_loop, outlet, timer_rx)
}

fn build_tool_engine_context(tool_packages: &Vec<String>) -> ToolEngineContext {
    let mut tool_pkgs = HashSet::new();
    for pkg_name in tool_packages {
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
    ToolEngineContext::new(tool_pkgs, &*TOOL_PACKAGE_TOOL)
}
async fn build_agent_context(identity: &AgentIdentity) -> Arc<AgentContext> {
    let agent_id = identity.id();
    let mut context = AgentContext::new();
    //初始化工作空间
    let space = std::env::current_dir().unwrap().join(&identity.workspace());
    if !space.exists() {
        std::fs::create_dir_all(space).unwrap();
    }
    //获取权限
    let permissions = {
        let permissions = entity_service::agent_permission::list(agent_id)
            .await
            .unwrap();
        let authority_context = AgentAuthorityContext::new();

        for item in permissions {
            let life: PermissionLife = serde_json::from_value(item.life).unwrap();

            let p = AgentPermission {
                id: item.id,
                code: item.code,
                sub_code: item.sub_code,
                life: life,
                purpose: item.purpose,
                use_times: AtomicU32::new(item.use_times),
                create_time: item.create_time,
            };
            authority_context.add(p)
        }
        authority_context
    };
    context.insert(permissions);
    //获取定时器
    let timers = {
        let timers = entity_service::agent_timer::list(agent_id).await.unwrap();
        let context = AgentSchedulerContext::new();

        for t in timers {
            let timer = AgentTimer {
                id: t.id,
                corn: t.corn,
                times: t.times,
                trigger_times: AtomicU32::new(t.trigger_times),
                payload: t.payload,
            };
            context.add(timer);
        }
        context
    };
    context.insert(timers);
    // 工具上下文
    let ctx = build_tool_engine_context(&identity.tool_packages);
    context.insert(ctx);
    //
    //注入相关工具
    {
        let injected = TimerInjected {
            add: Box::new(move |timer| {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        let id = agent.instance.scheduler.add(timer).await?;
                        return Ok(id);
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
            cancel: Box::new(move |id| {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        let _ = agent.instance.scheduler.cancel(id).await?;
                        return Ok(());
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
            sleep: Box::new(move |dur| {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        agent.heartbeat.disable();
                        println!("Sleeping...");
                        tokio::spawn(async move {
                            tokio::time::sleep(dur).await;
                            agent.heartbeat.enable();
                            println!("Wake up...");
                        });
                        return Ok(());
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
        };
        context.insert(injected);
        let injected = ReflectInjected {
            load_groups: Box::new(move |group_names| {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        let rule_engine = agent.reflect_engine.clone();
                        let groups = load_rule_groups(&rule_engine, &group_names).await?;
                        rule_engine.clear_groups();
                        for group in groups {
                            rule_engine.add_rule_group(group);
                        }
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
            unload_groups: Box::new(move |group_names| {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        let rule_engine = agent.reflect_engine.clone();
                        for group in group_names {
                            rule_engine.remove_rule_group(&group);
                        }
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
            current_group_names: Box::new(move || {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        let rule_engine = agent.reflect_engine.clone();
                        return Ok(rule_engine.group_names());
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
            simplify_expr: Box::new(move |expr| {
                let agent = {
                    let p = AGENT_POOL.read();
                    p.get(&agent_id).map(|v| v.clone())
                };

                async move {
                    if let Some(agent) = agent {
                        let rule_engine = agent.reflect_engine.clone();
                        let expr = rule_engine.simplify(&expr)?;
                        return Ok(expr);
                    }
                    anyhow::bail!("Agent not found")
                }
                .boxed()
            }),
        };
        context.insert(injected);
    }
    Arc::new(context)
}

fn build_agent_identity(agent: &entity::agent::Model) -> Arc<AgentIdentity> {
    let identity = AgentIdentity {
        id: agent.id,
        code: agent.code.to_string(),
        name: agent.name.to_string(),
        workspace: PathBuf::from(&agent.workspace),
        tool_packages: agent.tool_packages.0.clone(),
    };
    Arc::new(identity)
}
