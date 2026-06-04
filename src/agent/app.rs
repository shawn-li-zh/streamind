use std::sync::{atomic::AtomicU32, Arc};

use anyhow::Ok;

use entity_service::{event::event_convert, DB};

use futures::FutureExt as _;
use kernel::{
    agent::{context::AgentContext, AgentIdentity},
    mind::{
        context::EventContext as _,
        port::{CommandEvent, MindInlet},
        stream::StreamMind,
    },
    tool::engine::ToolEngineContext,
};

use reflection::engine::engine::RuleEngine;
use runtime::{
    agent::{
        authority::{AgentAuthorityContext, AgentPermission, PermissionLife},
        event::{AppEnvEvent, AppEnvEventData},
        reflect::{ReflectData, ReflectEvent},
        scheduler::{AgentSchedulerContext, AgentSchedulerHandler, AgentTimer},
    },
    context::stack::StackContext,
};

use tokio::sync::{watch, Mutex};
use tool_pkg::{reflect::ReflectInjected, timer::TimerInjected};
use tracing::{error, info};

use crate::{
    agent::{
        common::{build_agent_identity, build_tool_engine_context, WebSender},
        memory::MemoryAgent,
    },
    event_context::load_stack,
    global::{AGENT_POOL, EMBEDDING_CLIENT, ENGINE_POOL},
    plugin::{
        authority::AgentAuthority,
        heartbeat::{Heartbeat, HeartbeatRunner},
        logger::AgentLogger,
        reflect::{self, CondReflectEngine},
        scheduler::{AgentScheduler, AgentSchedulerRunner},
    },
    runtime::app::AppAgentRuntime,
    sse_event::WebEvent,
};
pub struct AppAgent {
    pub identity: Arc<AgentIdentity>,
    pub context: Arc<AgentContext>,
    pub event_context: Arc<StackContext<AppEnvEvent>>,
    pub mind_inlet: MindInlet,
    pub heartbeat: Arc<Heartbeat>,
    pub(crate) web_sender: Arc<WebSender>,
    gen_watcher: Mutex<watch::Receiver<bool>>,
    logger: Arc<AgentLogger>,
    scheduler: Arc<AgentScheduler>,
    authority: Arc<AgentAuthority>,
    pub(crate) memory_agent: Arc<MemoryAgent>,
    pub(crate) reflect_engine: CondReflectEngine,
}
impl AppAgent {
    pub async fn new(agent: entity::agent::Model) -> (Arc<Self>, AppAgentRuntime) {
        //chat 客户端
        let client = {
            let model_id = agent.model_id;
            let p = ENGINE_POOL.read();
            let client = p.get(&model_id).unwrap().clone();
            client
        };
        let (_model_family, _model_name, modals) = client.model_info();
        //
        let identity = Arc::new(build_agent_identity(&agent, modals));
        let context = build_agent_context(&identity).await;
        //事件上下文
        let event_context = load_stack(identity.as_ref(), client.clone(), &context)
            .await
            .unwrap();
        let event_context = Arc::new(event_context);
        //初始化工具上下文
        if let Some(ctx) = context.get::<ToolEngineContext>() {
            ctx.init(context.clone());
        }
        //mind
        let (stream_mind, mind_inlet, mind_outlet) = StreamMind::new(event_context.clone());
        let heartbeat = Arc::new(Heartbeat::new());
        let gen = watch::channel(false);
        //

        let reflect_engine: RuleEngine<ReflectData> =
            reflect::load_reflection_engine(&agent).await.unwrap();
        //日志
        let logger: AgentLogger = AgentLogger::new(&identity.workspace).await.unwrap();
        let logger = Arc::new(logger);

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
        //记忆任务
        let (memory_agent, memory_agent_runtime) =
            MemoryAgent::new(identity.clone(), context.clone(), client.clone()).await;
        let agent = AppAgent {
            identity,
            context,
            event_context: event_context.clone(),
            heartbeat: heartbeat.clone(),
            mind_inlet,
            web_sender: Arc::new(WebSender::new()),
            gen_watcher: Mutex::new(gen.1),
            logger: logger.clone(),
            reflect_engine: CondReflectEngine::new(agent.id, event_context, reflect_engine),
            memory_agent,
            scheduler: scheduler.clone(),
            authority: authority.clone(),
        };

        let agent = Arc::new(agent);

        //
        let heartbeat_runner = HeartbeatRunner::new(heartbeat, agent.clone());
        let runner = AgentSchedulerRunner::new(agent.clone(), timer_rx);
        let runtime = AppAgentRuntime::new(
            agent.clone(),
            stream_mind,
            mind_outlet,
            heartbeat_runner,
            runner,
            gen.0,
            memory_agent_runtime,
        );

        (agent, runtime)
    }
}

impl AppAgent {
    pub async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()> {
        //发送给代理
        self.mind_inlet.send(cmd).await?;
        Ok(())
    }
    pub async fn wait(&self, is_run: bool) -> anyhow::Result<()> {
        self.mind_inlet.wait(is_run).await?;
        Ok(())
    }
    pub async fn gen_wait(&self, is: bool) -> anyhow::Result<()> {
        let mut w = self.gen_watcher.lock().await;
        w.wait_for(|v| *v == is).await?;
        Ok(())
    }
    pub fn event_context(&self) -> Arc<StackContext<AppEnvEvent>> {
        self.event_context.clone()
    }
    pub fn context(&self) -> Arc<AgentContext> {
        self.context.clone()
    }
    pub fn identity(&self) -> Arc<AgentIdentity> {
        self.identity.clone()
    }
    pub async fn shutdown(&self) -> anyhow::Result<()> {
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
        self.heartbeat.disable();
        info!("AgentEvent: {:?}", &env_event.data);
        let reflection = self.reflect_engine.event_reflect(&env_event).await?;
        //发送给代理
        if env_event.force {
            self.send_cmd(CommandEvent::Cancel).await?;
        }

        //等待生成完成
        let mut gen = self.gen_watcher.lock().await;
        gen.wait_for(|v| !*v).await?;
        //发送事件给上下文
        self.inject_event_to_context(env_event, reflection).await?;
        //开始生成
        self.send_cmd(CommandEvent::Generate).await?;
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
            let ty = event_convert::to_type_enum(&data);
            let value = event_convert::to_json(&data);
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
        let event_context = &self.event_context;
        event_context.push_event(env_event);
        //
        if let Some(reflection) = reflection {
            let reflection = AppEnvEventData::Reflection(reflection);
            let event = self.save_event((reflection, false)).await?;
            self.logger.log_event(&event).await?;
            event_context.push_event(event);
        }
        Ok(())
    }
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
                        let id = agent.scheduler.add(timer).await?;
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
                        let _ = agent.scheduler.cancel(id).await?;
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
                            agent.heartbeat.reset();
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
            embedding: Arc::new(move |query| {
                async move {
                    let client = EMBEDDING_CLIENT.get().unwrap();
                    let embedding = client.union_embed(vec![query.into()]).await.map_err(|e| {
                        error!("embedding error: {}", e);
                        anyhow::anyhow!("embedding error")
                    })?;
                    Ok(embedding)
                }
                .boxed()
            }),

            simplify_expr: Arc::new(move |expr| {
                async move {
                    let agent = {
                        let p = AGENT_POOL.read();
                        p.get(&agent_id).map(|v| v.clone())
                    };

                    if let Some(agent) = agent {
                        let rule_engine = agent.reflect_engine.reflect_engine.clone();
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
