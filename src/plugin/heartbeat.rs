use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use ::runtime::agent::event::AppEnvEventData;
use tracing::error;

use crate::agent::app::AppAgent;

pub struct Heartbeat {
    flag: AtomicBool,
    in_progress: AtomicBool,
    enabled: AtomicBool,
}

impl Heartbeat {
    pub fn new() -> Self {
        let flag = AtomicBool::new(false);
        let in_progress = AtomicBool::new(false);
        let enabled = AtomicBool::new(true);
        Self {
            flag,
            in_progress,
            enabled,
        }
    }
    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    pub fn in_progress(&self) {
        self.in_progress.store(true, Ordering::SeqCst);
    }
    pub fn in_idle(&self) {
        self.flag.store(false, Ordering::SeqCst);
        self.in_progress.store(false, Ordering::SeqCst);
    }
    fn try_beat(&self) -> bool {
        let status = self.is_enabled();
        if !status {
            println!("heartbeat disabled, skipping..");
            return false;
        }
        if self.in_progress.load(Ordering::SeqCst) {
            println!("agent in progress, skipping...");
            return false;
        }

        if self
            .flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            println!("Previous heartbeat still pending, skipping...");
            return false;
        }
        true
    }
}
pub struct HeartbeatRunner {
    inner: Arc<Heartbeat>,
    agent: Arc<AppAgent>,
}
impl HeartbeatRunner {
    pub fn new(inner: Arc<Heartbeat>, agent: Arc<AppAgent>) -> HeartbeatRunner {
        Self {
            agent: agent.clone(),

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
            if !self.inner.try_beat() {
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
            println!("heartbeat...");
        }
    }
}
