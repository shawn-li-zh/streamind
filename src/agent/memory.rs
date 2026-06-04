use std::sync::{atomic::AtomicU32, Arc, LazyLock};

use anyhow::Ok;

use futures::FutureExt as _;
use kernel::{
    agent::{context::AgentContext, AgentIdentity},
    client::ChatClient,
    message::{Role, RoleMessage},
    mind::{
        context::EventContext as _,
        event::EnvEvent,
        port::{CommandEvent, MindInlet},
        section::Section,
        stream::StreamMind,
    },
    tool::engine::ToolEngineContext,
};

use runtime::{agent::event::AppEnvEvent, context::linear::LinearContext};

use tokio::sync::{watch, Mutex};
use tool_pkg::reflect::ReflectInjected;

use crate::{
    agent::common::build_tool_engine_context,
    event_context::{build_memory_system_prompt, load_stack},
    plugin::{
        logger::{self, AgentLogger},
        memory::{MemoryTaskManager, TaskDataItem},
    },
    runtime::memory::MemoryAgentRuntime,
};

pub struct MemoryMessage(RoleMessage);

impl EnvEvent for MemoryMessage {
    fn to_message(&self) -> RoleMessage {
        self.0.clone()
    }

    fn size(&self) -> usize {
        self.0.size()
    }
}

impl From<Vec<Section>> for MemoryMessage {
    fn from(value: Vec<Section>) -> Self {
        let s: String = value
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        MemoryMessage(RoleMessage::text(Role::Assistant, s))
    }
}

impl From<AppEnvEvent> for MemoryMessage {
    fn from(value: AppEnvEvent) -> Self {
        let msg = value.to_message();
        MemoryMessage(msg)
    }
}
impl From<RoleMessage> for MemoryMessage {
    fn from(value: RoleMessage) -> Self {
        MemoryMessage(value)
    }
}

static TOOLS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["event", "tool", "reflect"]
        .into_iter()
        .map(|v| v.to_string())
        .collect()
});
pub struct MemoryAgent {
    pub identity: Arc<AgentIdentity>,
    pub context: Arc<AgentContext>,
    event_context: Arc<LinearContext<MemoryMessage>>,
    mind_inlet: MindInlet,
    gen_watcher: Mutex<watch::Receiver<bool>>,
    logger: Arc<AgentLogger>,
    memory_task: Arc<MemoryTaskManager>,
}
impl MemoryAgent {
    pub async fn new(
        identity: Arc<AgentIdentity>,
        app_agent_context: Arc<AgentContext>,
        client: Arc<dyn ChatClient>,
    ) -> (Arc<Self>, MemoryAgentRuntime) {
        //初始化工具上下文
        let reflect_injectd = match app_agent_context.get::<ReflectInjected>() {
            Some(v) => (&*v).clone(),
            None => {
                panic!("没有找到反射注入器")
            }
        };
        let mut context: AgentContext = AgentContext::new();
        context.insert(reflect_injectd);
        let tool_engine_context = build_tool_engine_context(&*TOOLS);
        tool_engine_context.init(app_agent_context.clone());
        context.insert(tool_engine_context);
        let context = Arc::new(context);

        //事件上下文
        let system_prompt: String = build_memory_system_prompt(&identity, &context);
        let event_context = LinearContext::new(system_prompt, client);
        let event_context = Arc::new(event_context);

        //mind
        let (stream_mind, mind_inlet, mind_outlet) = StreamMind::new(event_context.clone());
        let gen = watch::channel(false);

        //日志
        let logger: AgentLogger = AgentLogger::new(&identity.workspace.join("memory"))
            .await
            .unwrap();
        let logger = Arc::new(logger);
        //记忆任务
        let (memory_task, memory_task_runner) = MemoryTaskManager::new(identity.clone());
        let agent = MemoryAgent {
            identity,
            context,
            event_context: event_context,
            mind_inlet: mind_inlet,
            gen_watcher: Mutex::new(gen.1),
            logger: logger.clone(),
            memory_task: Arc::new(memory_task),
        };
        let agent = Arc::new(agent);
        //
        let runtime = MemoryAgentRuntime::new(
            agent.clone(),
            stream_mind,
            mind_outlet,
            gen.0,
            memory_task_runner,
        );

        (agent, runtime)
    }

    pub async fn push_task(&self, task_data: Vec<TaskDataItem>) -> anyhow::Result<()> {
        self.memory_task.push_task(task_data).await
    }
    pub fn clear_events(&self) {
        self.event_context.clear();
    }
}

impl MemoryAgent {
    pub async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()> {
        //发送给代理
        self.mind_inlet.send(cmd).await?;
        Ok(())
    }

    pub async fn gen_wait(&self, is: bool) -> anyhow::Result<()> {
        let mut w = self.gen_watcher.lock().await;
        w.wait_for(|v| *v == is).await?;
        Ok(())
    }
    pub async fn push_message<T: Into<MemoryMessage>>(&self, msg: T) -> anyhow::Result<()> {
        let msg = msg.into();
        self.logger.log_event(&msg).await?;
        self.event_context.push_event(msg);
        Ok(())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let context = self.context.clone();
        if let Some(tool_engine_context) = context.get::<ToolEngineContext>() {
            tool_engine_context.shutdown().await?;
        }
        Ok(())
    }
}
