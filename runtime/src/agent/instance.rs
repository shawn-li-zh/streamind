use std::sync::Arc;

use kernel::{
    agent::{AgentIdentity, AgentOutfit, context::AgentContext},
    mind::{
        port::{CommandEvent, MindInlet, MindOutlet},
        stream::StreamMind,
    },
};

use crate::{agent::event::AppEnvEvent, context::stack::StackContext};

pub struct AgentInstance {
    pub identity: Arc<AgentIdentity>,
    pub context: Arc<AgentContext>,
    pub event_context: Arc<StackContext<AppEnvEvent>>,
    pub mind_inlet: MindInlet,
}

impl AgentInstance {
    pub fn build(
        identity: Arc<AgentIdentity>,
        context: Arc<AgentContext>,
        event_context: Arc<StackContext<AppEnvEvent>>,
    ) -> (AgentInstance, EventLoop, MindOutlet) {
        //通道
        //mind
        let (stream_mind, mind_inlet, mind_outlet) = StreamMind::new(event_context.clone());
        //创建代理实例
        let agent = AgentInstance {
            identity,
            event_context: event_context.clone(),
            context: context.clone(),
            mind_inlet,
        };
        let event_loop = EventLoop { stream_mind };

        (agent, event_loop, mind_outlet)
    }
}

impl AgentInstance {
    pub async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()> {
        let _ = self.mind_inlet.send(cmd).await?;
        Ok(())
    }
    pub async fn wait(&self, is_run: bool) -> anyhow::Result<()> {
        self.mind_inlet.wait(is_run).await?;
        Ok(())
    }
}

pub struct EventLoop {
    stream_mind: StreamMind<AppEnvEvent, StackContext<AppEnvEvent>>,
}
impl EventLoop {
    pub fn start(mut self) {
        tokio::spawn(async move { self.stream_mind.event_loop().await });
    }
}
