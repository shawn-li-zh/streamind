use std::sync::{Arc, LazyLock};

use chrono::Local;
use entity::task_queue::TaskStatus;
use entity_service::{event::event_convert, DB};
use futures::SinkExt;
use kernel::{
    agent::AgentIdentity,
    message::RoleMessage,
    mind::{context::EventContext, event::EnvEvent, port::CommandEvent},
};
use runtime::context::stack::{MindNode, Node};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::error;

use crate::global::AGENT_POOL;

pub struct TaskData {
    pub task_id: i64,
    pub node_context: Vec<TaskDataItem>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDataItem {
    pub node: Node,
    pub event_ids: Vec<i64>,
}

pub struct MemoryTaskManager {
    agent_identity: Arc<AgentIdentity>,
    sender: mpsc::Sender<TaskData>,
}

impl MemoryTaskManager {
    pub fn new(agent_identity: Arc<AgentIdentity>) -> (Self, MemoryTaskRunner) {
        let (sender, receiver) = mpsc::channel(100);
        let task_runner = MemoryTaskRunner::new(agent_identity.clone(), receiver, sender.clone());
        let s = Self {
            agent_identity,
            sender,
        };
        (s, task_runner)
    }
    pub async fn push_task(&self, task_data: Vec<TaskDataItem>) -> anyhow::Result<()> {
        let payload = serde_json::to_value(&task_data)?;
        let id = entity_service::task::new_task("memory", payload).await?;
        self.sender
            .send(TaskData {
                task_id: id,
                node_context: task_data,
            })
            .await?;
        Ok(())
    }
}

pub static REQUEST: LazyLock<String> =
    LazyLock::new(|| std::fs::read_to_string("prompts/memory/request.md").unwrap_or_default());

pub struct MemoryTaskRunner {
    agent_identity: Arc<AgentIdentity>,
    receiver: mpsc::Receiver<TaskData>,
    sender: mpsc::Sender<TaskData>,
}
impl MemoryTaskRunner {
    pub fn new(
        agent_identity: Arc<AgentIdentity>,
        receiver: mpsc::Receiver<TaskData>,
        sender: mpsc::Sender<TaskData>,
    ) -> Self {
        Self {
            agent_identity,
            receiver,
            sender,
        }
    }
    pub async fn start(self) -> anyhow::Result<()> {
        let tasks = entity_service::task::load_tasks("memory", TaskStatus::Pending).await?;
        for ele in tasks {
            let task_data = serde_json::from_value::<Vec<TaskDataItem>>(ele.payload)?;
            self.sender
                .send(TaskData {
                    task_id: ele.id,
                    node_context: task_data,
                })
                .await?;
        }
        tokio::spawn(async move {
            let mut runner = self;
            loop {
                match runner.run().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }
        });
        Ok(())
    }
    async fn run(&mut self) -> anyhow::Result<()> {
        let agent_id = self.agent_identity.id();
        let mem_agent = {
            let pool = AGENT_POOL.read();
            let agent = pool.get(&agent_id).unwrap();
            agent.memory_agent.clone()
        };
        let agent_id = mem_agent.identity.id();

        while let Some(task) = self.receiver.recv().await {
            println!("recv task:{}", task.task_id);
            entity_service::task::update_tasks(vec![task.task_id], TaskStatus::Processing).await?;
            // 清空上下文
            mem_agent.clear_events();
            //组织上下文
            let mut context = String::new();
            let mut top_pos = 0;
            for item in task.node_context {
                let node = item.node;
                top_pos = node.pos;
                context.push_str(node.statement().as_str());
                let events = entity_service::event::list_by_ids(agent_id, item.event_ids).await?;
                for ele in events {
                    let event = event_convert::from_entity(ele);
                    let msg = event.to_message();
                    let role = msg.role();
                    let (text, _paths) = msg.to_tuple();
                    context.push_str(&format!("\n{:?}:\n{}", role, text));
                }
            }
            let req = RoleMessage::User(vec![context.into()]);
            mem_agent.push_message(req).await?;
            let req = RoleMessage::User(vec![format!(
                "你只需要关注**当前栈顶pos={}**后面的事件和内容，将其转化为DSL,沉淀为可复用的经验和记忆",top_pos
            )
            .into()]);
            mem_agent.push_message(req).await?;
            //生成
            mem_agent.send_cmd(CommandEvent::Generate).await?;
            //先睡一秒
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            //等待生成完成
            mem_agent.gen_wait(false).await?;
            //更新任务进度
            entity_service::task::update_tasks(vec![task.task_id], TaskStatus::Done).await?;
        }
        Ok(())
    }
}
