use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use kernel::agent::{
    AgentIdentity, AgentOutfit,
    context::{AgentContext, AgentStatus},
    port::AgentOutlet,
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
impl AgentOutfit<AppEnvEvent, StackContext<AppEnvEvent>, ()> for AgentInstance {
    async fn send_event(&self, event: AppEnvEvent) -> anyhow::Result<()> {
        self.agent.send_event(event).await
    }
    // async fn send_status(&self, status: AgentStatus) -> anyhow::Result<()> {
    //     self.agent.send_status(status).await
    // }
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
