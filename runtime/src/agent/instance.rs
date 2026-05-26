use std::sync::Arc;

use async_trait::async_trait;
use kernel::{
    agent::{AgentIdentity, AgentOutfit, context::AgentContext, port::AgentOutlet},
    mind::port::CommandEvent,
};

use crate::agent::{
    authority::AgentAuthorityHandler, event::AppEnvEvent, event_context::StackContext,
    scheduler::AgentSchedulerHandler,
};
pub type Agent = kernel::agent::Agent<AppEnvEvent, StackContext<AppEnvEvent>>;
pub struct AgentInstance {
    agent: Arc<Agent>,
    pub scheduler: Arc<dyn AgentSchedulerHandler>,
    pub authority: Arc<dyn AgentAuthorityHandler>,
}

#[async_trait]
impl AgentOutfit<AppEnvEvent, StackContext<AppEnvEvent>> for AgentInstance {
    async fn send_cmd(&self, cmd: CommandEvent<AppEnvEvent>) -> anyhow::Result<()> {
        self.agent.send_cmd(cmd).await
    }
    fn event_context(&self) -> Arc<StackContext<AppEnvEvent>> {
        self.agent.event_context()
    }
    fn context(&self) -> Arc<AgentContext> {
        self.agent.context()
    }
    fn identity(&self) -> Arc<AgentIdentity> {
        self.agent.identity()
    }
    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
pub type EventLoop = kernel::agent::EventLoop<AppEnvEvent, StackContext<AppEnvEvent>>;
pub fn build_agent_instance(
    identity: Arc<AgentIdentity>,
    context: Arc<AgentContext>,
    event_context: Arc<StackContext<AppEnvEvent>>,
    scheduler: Arc<dyn AgentSchedulerHandler>,
    authority: Arc<dyn AgentAuthorityHandler>,
) -> (AgentInstance, EventLoop, AgentOutlet<AppEnvEvent>) {
    let (agent, event_loop, outlet) = kernel::agent::Agent::build(identity, context, event_context);
    // let context = agent.context();
    let ins = AgentInstance {
        agent: Arc::new(agent),
        scheduler,
        authority,
    };
    (ins, event_loop, outlet)
}
