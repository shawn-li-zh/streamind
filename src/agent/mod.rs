use tokio::sync::mpsc;

use crate::{event::AgentEvent, stream::{LLMStream, MindStream}};
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: u32,
    pub name: String,
    pub workspace: String,
}
pub struct Agent {
    pub info: AgentInfo,
    sender: mpsc::Sender<AgentEvent>,
}
impl Agent {
    pub fn new(info: AgentInfo) -> Agent {
        let (sender, receiver) = mpsc::channel(100);

        // 启动后台循环
        let mut worker = AgentWorker::new(info.clone(), receiver);
        tokio::spawn(async move {
            worker.run().await;
        });
        Agent { info, sender }
    }
    pub async fn send_event(&self, event: AgentEvent) -> anyhow::Result<()> {
        self.sender.send(event).await?;
        Ok(())
    }
}

pub struct AgentWorker {
    pub info: AgentInfo,
    receiver: mpsc::Receiver<AgentEvent>,
    streams: Vec<MindStream>,
}
impl AgentWorker {
    pub fn new(info: AgentInfo, receiver: mpsc::Receiver<AgentEvent>) -> Self {
        Self { info, receiver, streams: Vec::new() }
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.receiver.recv().await {
            // Handle the received event
        }
    }
}
