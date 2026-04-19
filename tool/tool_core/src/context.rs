use std::{
    any::Any,
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use dashmap::DashMap;

use crate::ToolPackage;

pub struct ToolPackageContext {
    pub data: Arc<RwLock<dyn Any + Send + Sync + 'static>>,
    tool_limiter: DashMap<&'static str, RateLimiter>,
}

impl ToolPackageContext {
    pub fn new<T: Send + Sync + 'static>(data: T, pkg: &ToolPackage) -> Self {
        let map = DashMap::new();
        for (name, tool) in pkg.tools() {
            if let Some(rate_limit) = tool.rate_limit {
                map.insert(*name, RateLimiter::new(rate_limit.0, rate_limit.1));
            }
        }
        Self {
            data: Arc::new(RwLock::new(data)),
            tool_limiter: map,
        }
    }
    pub fn data(&self) -> std::sync::Arc<std::sync::RwLock<dyn Any>> {
        self.data.clone()
    }

    pub fn is_limited(&self, tool_name: &str) -> bool {
        if let Some(counter) = self.tool_limiter.get(tool_name) {
            return counter.value().check();
        }
        false
    }
}

pub struct RateLimiter {
    slots: Mutex<VecDeque<Instant>>,
    window_size: Duration,
    limit_times: usize,
}

impl RateLimiter {
    pub fn new(window_size: Duration, limit_times: u32) -> Self {
        Self {
            slots: Mutex::new(VecDeque::new()),
            window_size,
            limit_times: limit_times as usize,
        }
    }

    pub fn check(&self) -> bool {
        let mut slots = self.slots.lock().unwrap();
        let now = Instant::now();
        // 移除窗口外的陈旧数据
        self.remove_past(&now, &mut slots);
        let total = slots.len();
        if total >= self.limit_times {
            return false;
        }
        // 记录当前时刻的计数
        slots.push_back(now);
        true
    }

    fn remove_past(&self, now: &Instant, slots: &mut VecDeque<Instant>) {
        while let Some((time)) = slots.front() {
            if now.duration_since(*time) > self.window_size {
                slots.pop_front();
            } else {
                break;
            }
        }
    }
}
