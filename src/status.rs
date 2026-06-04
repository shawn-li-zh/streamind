use std::sync::Arc;

use entity_service::DB;
use kernel::agent::{AgentIdentity, AgentOutfit, context::AgentStatus};
use runtime::agent::instance::AgentInstance;
use tokio::sync::{RwLock, watch};

pub struct AgentStatusHolder {
    agent: Arc<AgentInstance>,
    identity: Arc<AgentIdentity>,
    status: RwLock<AgentStatus>,
    sender: watch::Sender<AgentStatus>,
    receiver: RwLock<watch::Receiver<AgentStatus>>,
}

impl AgentStatusHolder {
    pub fn new(agent: Arc<AgentInstance>) -> Self {
        let identity = agent.identity.clone();
        let (sender, receiver) = watch::channel(AgentStatus::Idle);
        Self {
            agent,
            identity,
            status: RwLock::new(AgentStatus::Idle),
            sender,
            receiver: RwLock::new(receiver),
        }
    }
    pub async fn get_status(&self) -> AgentStatus {
        let s = self.status.read().await;
        s.clone()
    }

    pub async fn send_status(&self, status: AgentStatus) -> anyhow::Result<()> {
        // let mut status_ref = self.status.write().await;
        // *status_ref = status;
        // self.sender.send(status)?;
        // println!("状态更新:{:?}", status);
        // let db = DB.get().unwrap();
        // entity_service::agent::update_status(db, self.identity.id(), status).await?;
        Ok(())
    }
  
     pub async fn send_status_if<F:Fn(AgentStatus)->bool>(
        &self,
        status: AgentStatus,
        test:F
    ) -> anyhow::Result<bool> {
        // let mut status_ref = self.status.write().await;
        // if !test(*status_ref) {
        //     return Ok(false);
        // }
        // *status_ref = status;
        // self.sender.send(status)?;
        // println!("状态更新:{:?}", status);
        // let db = DB.get().unwrap();
        // entity_service::agent::update_status(db, self.identity.id(), status).await?;
        Ok(true)
    }
    pub async fn wait_status_is(&self, is: AgentStatus) -> anyhow::Result<()> {
        let mut receiver: tokio::sync::RwLockWriteGuard<'_, watch::Receiver<AgentStatus>> = self.receiver.write().await;
        receiver.wait_for(|v| *v == is).await?;
        Ok(())
    }
    pub async fn wait_status_not_is(&self, not_is: AgentStatus) -> anyhow::Result<()> {
        let mut receiver = self.receiver.write().await;
        receiver.wait_for(|v| *v != not_is).await?;
        Ok(())
    }
}
