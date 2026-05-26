pub mod context;
pub mod error;
pub mod event;

pub mod port;

use std::{marker::PhantomData, path::PathBuf, sync::Arc};

use crate::{
    agent::{
        event::{AgentEvent, EnvEvent},
        port::AgentOutlet,
    },
    mind::{
        context::EventContext,
        port::{CommandEvent, MindInlet, MindOutlet},
        stream::{MindFlag, StreamMind},
    },
};

use async_trait::async_trait;
use futures::StreamExt;

use tokio::sync::mpsc;

use crate::agent::context::AgentContext;
pub struct AgentIdentity {
    pub id: u32,
    pub code: String,
    pub name: String,
    pub workspace: PathBuf,
    pub tool_packages: Vec<String>,
}
impl AgentIdentity {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn code(&self) -> &str {
        self.code.as_str()
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn workspace(&self) -> PathBuf {
        self.workspace.clone()
    }
}
#[async_trait]
pub trait AgentOutfit<E, C>: Send + Sync + 'static {
    async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()>;

    fn event_context(&self) -> Arc<C>;
    fn context(&self) -> Arc<AgentContext>;
    fn identity(&self) -> Arc<AgentIdentity>;
    async fn shutdown(&self) -> anyhow::Result<()>;
}

pub struct Agent<E: EnvEvent, C: EventContext<E>> {
    identity: Arc<AgentIdentity>,
    context: Arc<AgentContext>,
    event_context: Arc<C>,
    mind_inlet: MindInlet,
    __phantom: PhantomData<E>,
}

impl<E: EnvEvent, C: EventContext<E>> Agent<E, C> {
    pub fn build(
        identity: Arc<AgentIdentity>,
        context: Arc<AgentContext>,
        event_context: Arc<C>,
    ) -> (Agent<E, C>, EventLoop<E, C>, MindOutlet) {
        //通道
        //mind
        let (stream_mind, mind_inlet, mind_outlet) = StreamMind::new(event_context.clone());
        //创建代理实例
        let agent = Agent {
            identity,
            event_context: event_context.clone(),
            context: context.clone(),
            mind_inlet,
            __phantom: PhantomData,
        };
        let event_loop = EventLoop { stream_mind };

        (agent, event_loop, mind_outlet)
    }
}

#[async_trait]
impl<E: EnvEvent, C: EventContext<E>> AgentOutfit<E, C> for Agent<E, C> {
    async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()> {
        self.mind_inlet.send(cmd).await?;
        Ok(())
    }
    // async fn send_status(&self, status: AgentStatus) -> anyhow::Result<()> {
    //     self.event_sender.send(AgentEvent::Status(status)).await?;
    //     Ok(())
    // }
    fn event_context(&self) -> Arc<C> {
        self.event_context.clone()
    }
    fn context(&self) -> Arc<AgentContext> {
        self.context.clone()
    }
    fn identity(&self) -> Arc<AgentIdentity> {
        self.identity.clone()
    }
    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct EventLoop<E: EnvEvent, C: EventContext<E>> {
    stream_mind: StreamMind<E, C>,
}
impl<E: EnvEvent, C: EventContext<E>> EventLoop<E, C> {
    pub fn start(mut self) {
        tokio::spawn(async move { self.stream_mind.event_loop().await });
    }
}
