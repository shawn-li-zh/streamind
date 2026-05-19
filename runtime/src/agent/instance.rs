use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicPtr, AtomicU32},
    },
    time::{Duration, Instant},
};

use async_trait::async_trait;
use dashmap::DashMap;

use entity::agent;
use entity_service::DB;
use kernel::{
    agent::{
        AgentHandler,
        context::{AgentContext, AgentEvent, AgentInfo, AgentStatus, AgentTimer, PermissionLife},
    },
    event::EnvEvent,
    tool::engine::{ToolEngine, ToolEngineContext},
};

use tokio::{
    sync::{RwLock, mpsc, watch},
    time::sleep_until,
};

use crate::{
    agent::{
        channel::AgentChannel,
        permission::AgentAuthority,
        runtime::AgentRuntime,
        scheduler::{self, AgentScheduler},
    },
    context::mind::StreamMind,
    global::{AGENT_CTX_POOL, ENGINE_POOL, TOOL_PACKAGE_TOOL},
    kit::message_kit,
};

pub struct AgentInstance {
    pub context: Arc<AgentContext>,
    pub channel: Arc<AgentChannel>,
    pub scheduler: Arc<AgentScheduler>,
    pub authority: Arc<AgentAuthority>,
}

impl AgentInstance {
    pub async fn build(
        agent: entity::agent::Model,
    ) -> (Arc<AgentInstance>, AgentRuntime, StreamMind) {
        let agent_id = agent.id;
        let agent_name = agent.name.clone();
        let model_id = agent.model_id;
        //
        let context = build_agent_context(agent).await;
        //通道
        let event = mpsc::channel(100);
        let section = mpsc::channel(100);
        let inject: (
            mpsc::UnboundedSender<(String, bool)>,
            mpsc::UnboundedReceiver<(String, bool)>,
        ) = mpsc::unbounded_channel();

        let run_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        let status_watcher: (watch::Sender<AgentStatus>, watch::Receiver<AgentStatus>) =
            watch::channel(AgentStatus::Idle);

        //初始化
        let channel = AgentChannel::new(
            context.info.clone(),
            event.0,
            status_watcher.0,
            RwLock::new(run_watcher.1),
            RwLock::new(status_watcher.1),
        );

        let channel = Arc::new(channel);
        //上下文流处理器
        let client = {
            let p = ENGINE_POOL.read().await;
            let client = p.get(&model_id).unwrap().clone();
            client
        };
        let stack = message_kit::load_stack(&context).await.unwrap();
        let stack = Arc::new(RwLock::new(stack));
        let stream_mind = StreamMind::new(
            agent_id,
            client,
            stack.clone(),
            inject.1,
            event.1,
            section.0,
            run_watcher.0,
        );
        //定时器
        let scheduler = AgentScheduler::new(context.clone(), channel.clone());
        let authority: Arc<AgentAuthority> = Arc::new(AgentAuthority {
            context: context.clone(),
        });
        //创建代理实例
        let agent = AgentInstance {
            context: context.clone(),
            channel: channel.clone(),
            authority: authority.clone(),
            scheduler: Arc::new(scheduler),
        };
        let agent = Arc::new(agent);
        //工具执行器
        let tool_engine = { ToolEngine::new(agent.clone(), context.tool_engine_context.clone()) };
        //运行时
        let runtime = AgentRuntime::new(
            context.clone(),
            channel.clone(),
            stack.clone(),
            tool_engine,
            section.1,
        );

        (agent, runtime, stream_mind)
    }
    pub async fn stop(&self) -> anyhow::Result<()> {
        self.channel
            .send_event(EnvEvent::Interrupt("程序暂停".to_string()), true)
            .await?;
        self.channel.wait_run_is(false).await?;
        Ok(())
    }
}
impl Drop for AgentInstance {
    fn drop(&mut self) {
        let ctx = self.context.clone();
        tokio::spawn(async move { ctx.tool_engine_context.shutdown().await.unwrap() });
    }
}
#[async_trait]
impl AgentHandler for AgentInstance {
    async fn send_event(&self, event: EnvEvent, force: bool) -> anyhow::Result<AgentEvent> {
        let (event, _) = self.channel.send_event(event, force).await?;
        Ok(event)
    }

    async fn sleep(&self, duration: Duration) -> anyhow::Result<()> {
        let ok = self
            .channel
            .send_status_if(AgentStatus::Waiting, AgentStatus::Sleep)
            .await?;
        if !ok {
            anyhow::bail!("状态异常,睡眠失败");
        }
        //修改状态
        println!("sleep...");
        // 休眠中
        let ch = self.channel.clone();
        tokio::spawn(async move {
            let deadline = Instant::now() + duration;

            sleep_until(deadline.into()).await;
            println!("wake up...");
            //发送状态
            ch.send_status(AgentStatus::Idle).await.unwrap();
        });
        Ok(())
    }

    async fn wakeup(&self) -> anyhow::Result<()> {
        println!("wakeup...");
        self.channel
            .send_status_if(AgentStatus::Sleep, AgentStatus::Idle)
            .await?;
        let db = DB.get().unwrap();

        entity_service::agent::update_status(db, self.context.agent_id, AgentStatus::Idle).await?;
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.channel
            .send_event(EnvEvent::Interrupt("程序暂停".to_string()), true)
            .await?;
        self.channel.wait_run_is(false).await?;
        let db = DB.get().unwrap();
        let info = self.context.info.read().await;
        entity_service::agent::update_status(db, info.id, AgentStatus::Paused).await?;
        Ok(())
    }

    async fn get_status(&self) -> AgentStatus {
        let info = self.context.info.read().await;
        info.status.clone()
    }

    fn context(&self) -> Arc<AgentContext> {
        self.context.clone()
    }

    fn authority(&self) -> Arc<dyn kernel::agent::AgentAuthorityHandler> {
        self.authority.clone()
    }

    fn scheduler(&self) -> Arc<dyn kernel::agent::AgentSchedulerHandler> {
        self.scheduler.clone()
    }
}

pub async fn build_agent_context(agent: entity::agent::Model) -> Arc<AgentContext> {
    let pool = ENGINE_POOL.read().await;
    let client = pool.get(&agent.model_id).expect("模型不存在");
    let (model_family, model_name) = client.model_info();
    let agent_info = AgentInfo {
        id: agent.id,
        code: agent.code,
        model_id: agent.model_id,
        parent_id: agent.parent_id,
        state_id: agent.state_id,
        // state_pos: model.state_pos,
        name: agent.name,
        model: model_name,
        workspace: PathBuf::from(&agent.workspace),
        avatar: agent.avatar,
        status: serde_json::from_value(serde_json::to_value(agent.status).unwrap()).unwrap(),
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
        ToolEngineContext::new(tool_pkgs, &*TOOL_PACKAGE_TOOL)
    };
    //获取权限
    let permissions = {
        let permissions = entity_service::agent_permission::list(agent.id)
            .await
            .unwrap();
        let map = DashMap::new();
        for item in permissions {
            let life: PermissionLife = serde_json::from_value(item.life).unwrap();

            let p = kernel::agent::context::AgentPermission {
                id: item.id,
                code: item.code,
                sub_code: item.sub_code,
                life: life,
                purpose: item.purpose,
                use_times: item.use_times,
                create_time: item.create_time,
            };
            map.insert(p.code.clone(), p);
        }
        map
    };
    //获取定时器
    let timers = {
        let timers = entity_service::agent_timer::list(agent.id).await.unwrap();
        let map = DashMap::new();
        for t in timers {
            let timer = AgentTimer {
                id: t.id,
                corn: t.corn,
                times: t.times,
                trigger_times: AtomicU32::new(t.trigger_times),
                payload: t.payload,
            };
            map.insert(t.id, Arc::new(timer));
        }
        map
    };
    let agent_context = AgentContext {
        agent_id: agent.id,
        agent_code: agent_info.code.clone(),
        worksapce: agent_info.workspace.clone(),
        info: Arc::new(RwLock::new(agent_info)),
        tool_engine_context: Arc::new(tool_engine_context),
        permissions: Arc::new(permissions),
        timers: Arc::new(timers),
    };
    let agent_context = Arc::new(agent_context);
    agent_context
        .tool_engine_context
        .init(agent_context.clone());
    //
    let id = agent.id;

    //
    let mut p = AGENT_CTX_POOL.write().await;
    p.insert(id, agent_context.clone());
    agent_context
}
