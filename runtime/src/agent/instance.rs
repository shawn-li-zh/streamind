use std::{collections::HashSet, path::PathBuf, sync::Arc};

use dashmap::DashMap;

use entity_service::DB;
// use sea_orm::{DbErr, TransactionTrait};
use kernel::{
    agent::{AgentContext, AgentInfo, PermissionLife},
    event::EnvEvent,
    tool::engine::{ToolEngine, ToolEngineContext},
};

use tokio::sync::{RwLock, mpsc, watch};
use tracing::info;

use crate::{
    agent::{channel::AgentChannel, runtime::AgentRuntime},
    context::mind::{self, StreamMind},
    global::{AGENT_CTX_POOL, ENGINE_POOL, TOOL_PACKAGE_TOOL},
    kit::message_kit,
};

pub struct AgentInstance {
    pub context: Arc<AgentContext>,
    pub channel: Arc<AgentChannel>,
}

impl AgentInstance {
    pub async fn new(
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
        let call_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);
        let reorganize_watcher: (watch::Sender<bool>, watch::Receiver<bool>) =
            watch::channel(false);
        let run_watcher: (watch::Sender<bool>, watch::Receiver<bool>) = watch::channel(false);

        //工具执行器
        let tool_engine = { ToolEngine::new(context.clone(), context.tool_engine_context.clone()) };
        //初始化
        let channel = AgentChannel::new(
            context.info.clone(),
            event.0,
            RwLock::new(call_watcher.1),
            RwLock::new(run_watcher.1),
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
            reorganize_watcher.0,
            run_watcher.0,
        );
        //运行时
        let runtime = AgentRuntime::new(
            context.clone(),
            channel.clone(),
            stack.clone(),
            tool_engine,
            section.1,
            call_watcher.0,
        );

        let agent = AgentInstance {
            context: context.clone(),
            channel: channel.clone(),
        };
        let agent = Arc::new(agent);
        (agent, runtime, stream_mind)
    }
    pub async fn stop(&mut self, reason: String) -> anyhow::Result<()> {
        self.channel
            .send_event(EnvEvent::Interrupt(reason), true)
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
        sleep_until: agent.sleep_until,
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

            let p = kernel::agent::AgentPermission {
                id: item.id,
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
        agent_id: agent.id,
        agent_code: agent_info.code.clone(),
        worksapce: agent_info.workspace.clone(),
        info: Arc::new(RwLock::new(agent_info)),
        tool_engine_context: Arc::new(tool_engine_context),
        permissions: Arc::new(permissions),
    };
    agent_context
        .tool_engine_context
        .init(agent_context.clone());
    //
    let id = agent.id;

    let agent_context = Arc::new(agent_context);
    //
    let mut p = AGENT_CTX_POOL.write().await;
    p.insert(id, agent_context.clone());
    agent_context
}
