
use async_trait::async_trait;
use std::sync::{Arc, atomic::AtomicU32};

use dashmap::DashMap;

#[derive(Debug)]
pub struct AgentTimer {
    pub id: i64,
    pub corn: String,
    pub times: u32,
    pub payload: String,
    pub trigger_times: AtomicU32,
}
impl AgentTimer {
    pub fn new(corn: String, times: u32, payload: String) -> Self {
        Self {
            id: 0,
            corn,
            times,
            payload,
            trigger_times: AtomicU32::new(0),
        }
    }

    pub fn is_disused(&self) -> bool {
        if self.times == 0 {
            return false;
        }
        self.trigger_times
            .load(std::sync::atomic::Ordering::Relaxed)
            >= self.times
    }
}

pub struct AgentSchedulerContext {
    timers: DashMap<i64, Arc<AgentTimer>>,
}
impl AgentSchedulerContext {
    pub fn new() -> Self {
        Self {
            timers: DashMap::new(),
        }
    }
    pub fn add(&self, t: AgentTimer) -> Arc<AgentTimer> {
        let t = Arc::new(t);
        self.timers.insert(t.id, t.clone());
        t
    }
    pub fn remove(&self, id: i64) -> Option<Arc<AgentTimer>> {
        self.timers.remove(&id).map(|v| v.1)
    }
    pub fn contains_key(&self, id: i64) -> bool {
        self.timers.contains_key(&id)
    }
    pub fn iter(&self) -> dashmap::iter::Iter<'_, i64, Arc<AgentTimer>> {
        self.timers.iter()
    }
}

#[async_trait]
pub trait AgentSchedulerHandler: Send + Sync + 'static {
    //添加定时任务
    async fn add(&self, timer: AgentTimer) -> anyhow::Result<i64>;
    //取消定时任务
    async fn cancel(&self, id: i64) -> anyhow::Result<()>;
}
