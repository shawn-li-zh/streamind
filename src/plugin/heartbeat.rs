use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use kernel::agent::{context::AgentStatus, AgentOutfit};

use ::runtime::agent::event::AppEnvEventData;
use tracing::error;

use crate::{agent::AppAgent, status::AgentStatusHolder};
pub struct Heartbeat {
    flag: AtomicBool,
}

pub struct HeartbeatRunner {
    inner: Arc<Heartbeat>,
    agent: Arc<AppAgent>,
    status: Arc<AgentStatusHolder>,
}
impl Heartbeat {
    pub fn new() -> Self {
        let flag = AtomicBool::new(false);
        Self { flag }
    }

    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

impl HeartbeatRunner {
    pub fn new(
        inner: Arc<Heartbeat>,
        agent: Arc<AppAgent>,
        status: Arc<AgentStatusHolder>,
    ) -> HeartbeatRunner {
        Self {
            agent: agent.clone(),
            status: status.clone(),
            inner: inner,
        }
    }
    pub fn start(self, interval: Duration) {
        tokio::spawn(async move {
            self.beat(interval).await;
        });
    }
    async fn beat(&self, interval: Duration) {
        let mut interval = tokio::time::interval(interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            //只有空闲时才发送心跳
            let status = self.status.get_status().await;
            if status != AgentStatus::Idle {
                continue;
            }
            if self
                .inner
                .flag
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                println!("Previous heartbeat still pending, skipping...");
                continue;
            }
            if let Err(e) = self
                .agent
                .send_event((AppEnvEventData::Heartbeat, false))
                .await
            {
                error!("Heartbeat failed: {}", e);
                return;
            };
            println!("heartbeat...{:?}", status);
        }
    }
}
