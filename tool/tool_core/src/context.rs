use std::{
    any::Any,
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

use dashmap::DashMap;
pub trait PackageCtx: Send + Sync + Any {
    fn as_any(&self) -> &dyn Any;
}

pub struct ToolPackageContext<T> {
    pub data: T,
    tool_limiter: DashMap<&'static str, SlidingWindowCounter>,
}
impl<T: Send + Sync + 'static> PackageCtx for ToolPackageContext<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T> ToolPackageContext<T> {
    pub fn is_limited(&self, tool_name: &str, count: usize) -> bool {
        if let Some(counter) = self.tool_limiter.get(tool_name) {
            if counter.get_total() >= count {
                return true;
            }
            counter.add(1);
        }
        false
    }
}
pub struct ToolContext {
    pub packages: DashMap<&'static str, Box<dyn PackageCtx>>,
}

pub struct SlidingWindowCounter {
    slots: Mutex<VecDeque<(Instant, usize)>>,
    window_size: Duration,
}

impl SlidingWindowCounter {
    pub fn new(window_size: Duration) -> Self {
        Self {
            slots: Mutex::new(VecDeque::new()),
            window_size,
        }
    }

    pub fn add(&self, count: usize) {
        let mut slots = self.slots.lock().unwrap();
        let now = Instant::now();

        // 记录当前时刻的计数
        slots.push_back((now, count));

        // 移除窗口外的陈旧数据
        self.remove_past(&now, &mut slots);
    }

    pub fn get_total(&self) -> usize {
        let mut slots = self.slots.lock().unwrap();
        let now = Instant::now();

        self.remove_past(&now, &mut slots);
        // 求和
        slots.iter().map(|(_, count)| count).sum()
    }
    fn remove_past(&self, now: &Instant, slots: &mut VecDeque<(Instant, usize)>) {
        while let Some((time, _)) = slots.front() {
            if now.duration_since(*time) > self.window_size {
                slots.pop_front();
            } else {
                break;
            }
        }
    }
}
