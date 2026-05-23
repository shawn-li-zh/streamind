pub mod context;
pub mod error;
pub mod event;

pub mod port;

use std::{path::PathBuf, sync::Arc};

use crate::{
    agent::{
        event::{AgentEvent, EnvEvent},
        port::AgentOutlet,
    },
    mind::{
        MindInlet, MindOutlet,
        context::EventContext,
        stream::{MindFlag, StreamMind},
    },
    tool::engine::ToolEngineContext,
};

use async_trait::async_trait;
use futures::StreamExt;

use tokio::sync::mpsc;

use crate::agent::context::{AgentContext, AgentStatus};
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
pub trait AgentOutfit<E, C, ER> {
    async fn send_event(&self, event: E) -> anyhow::Result<ER>;
    // async fn send_status(&self, status: S) -> anyhow::Result<()>;
    fn event_context(&self) -> Arc<C>;
    fn context(&self) -> Arc<AgentContext>;
    fn identity(&self) -> Arc<AgentIdentity>;
    async fn shutdown(&self) -> anyhow::Result<()>;
}

pub struct Agent<E: EnvEvent, C: EventContext<E>> {
    identity: Arc<AgentIdentity>,
    context: Arc<AgentContext>,
    event_context: Arc<C>,
    mind_inlet: MindInlet<E>,
    event_sender: mpsc::Sender<AgentEvent<E>>,
}

impl<E: EnvEvent, C: EventContext<E>> Agent<E, C> {
    pub fn build(
        identity: Arc<AgentIdentity>,
        context: Arc<AgentContext>,
        event_context: Arc<C>,
    ) -> (Agent<E, C>, EventLoop<E, C>, AgentOutlet<E>) {
        //通道
        let agent_event = mpsc::channel(100);
        //mind
        let (stream_mind, mind_inlet, mind_outlet) = StreamMind::new(event_context.clone());
        //创建代理实例
        let agent = Agent {
            identity,
            event_context: event_context.clone(),
            context: context.clone(),
            mind_inlet,
            event_sender: agent_event.0.clone(),
        };
        let event_loop = EventLoop {
            stream_mind,
            mind_outlet,
            event_sender: agent_event.0,
        };
        let agent_outlet = AgentOutlet::new(agent_event.1);
        (agent, event_loop, agent_outlet)
    }
}

#[async_trait]
impl<E: EnvEvent, C: EventContext<E>> AgentOutfit<E, C, ()> for Agent<E, C> {
    async fn send_event(&self, event: E) -> anyhow::Result<()> {
        self.mind_inlet.send(event).await?;
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
    mind_outlet: MindOutlet,
    event_sender: mpsc::Sender<AgentEvent<E>>,
}
impl<E: EnvEvent, C: EventContext<E>> EventLoop<E, C> {
    pub fn start(mut self) {
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            loop {
                match self.stream_mind.event_loop().await.unwrap() {
                    MindFlag::Closed => return,
                    MindFlag::Overflow => {
                        //让agent退栈
                        // 发送失败说明 Agent 已销毁，直接退出即可
                        if self.event_sender.send(AgentEvent::Overflow).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });
        tokio::spawn(async move {
            while let Some(e) = self.mind_outlet.next().await {
                if sender.send(AgentEvent::Generate(e)).await.is_err() {
                    break; // 接收端已关闭，优雅退出
                }
            }
        });
    }
}
