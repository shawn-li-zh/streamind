use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use chrono::Local;

use crate::{
    agent::{AgentContext, AgentInfo},
    simple_id,
    tool::{
        Tool, ToolPackage,
        call::{ToolCall, new_temp_result_file},
    },
};

pub trait ToolCallnterceptor {
    fn build(agent: &AgentInfo, pkg: &'static ToolPackage, tool: &'static Tool) -> Option<Self>
    where
        Self: Sized;
    fn on_call(&self, call: &ToolCall) -> anyhow::Result<()> {
        Ok(())
    }
    //调用成功后，对消息进行处理
    fn on_success(&self, call: &ToolCall, content: String) -> String {
        content
    }
}
//========限流
pub struct RateLimiter {
    slots: Mutex<VecDeque<Instant>>,
    window_time: Duration,
    limit_times: usize,
}

impl RateLimiter {
    pub fn new(window_time: Duration, limit_times: u32) -> Self {
        Self {
            slots: Mutex::new(VecDeque::new()),
            window_time: window_time,
            limit_times: limit_times as usize,
        }
    }

    pub fn is_limited(&self) -> bool {
        let mut slots = self.slots.lock().unwrap();
        let now = Instant::now();
        // 移除窗口外的陈旧数据
        self.remove_past(&now, &mut slots);
        let total = slots.len();
        if total >= self.limit_times {
            return true;
        }
        // 记录当前时刻的计数
        slots.push_back(now);
        false
    }

    fn remove_past(&self, now: &Instant, slots: &mut VecDeque<Instant>) {
        while let Some((time)) = slots.front() {
            if now.duration_since(*time) > self.window_time {
                slots.pop_front();
            } else {
                break;
            }
        }
    }
}

impl ToolCallnterceptor for RateLimiter {
    fn build(agent: &AgentInfo, pkg: &'static ToolPackage, tool: &'static Tool) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(rate_limit) = tool.rate_limit {
            Some(RateLimiter::new(rate_limit.0, rate_limit.1))
        } else {
            None
        }
    }
    fn on_call(&self, call: &ToolCall) -> anyhow::Result<()> {
        if self.is_limited() {
            return Err(anyhow::anyhow!("调用频率过高"));
        }
        return Ok(());
    }
}

//=========返回结果进入上下文前拦截
pub struct ContentSizeLimiter {
    max_size: usize,
    workspace: PathBuf,
}
impl ToolCallnterceptor for ContentSizeLimiter {
    fn build(agent: &AgentInfo, pkg: &'static ToolPackage, tool: &'static Tool) -> Option<Self>
    where
        Self: Sized,
    {
        let max_size = std::env::var("TOOL_RESULT_SIZE_LIMIT")
            .map(|v| usize::from_str_radix(&v, 10).expect("env TOOL_RESULT_SIZE_LIMIT"))
            .unwrap_or(1024 * 8);

        Some(ContentSizeLimiter {
            workspace: agent.workspace.clone(),
            max_size,
        })
    }

    fn on_success(&self, call: &ToolCall, content: String) -> String {
        if content.len() >= self.max_size {
            let (path, mut file) =
                new_temp_result_file(&self.workspace, call.package.name, call.tool.name, "txt");
            file.write_all(content.as_bytes()).expect("文件写入失败");
            let path = split_after_folder(&path, "temp_result").unwrap();
            return format!("返回内容过大,已写入文件<file:{}>", path.to_str().unwrap());
        }
        content
    }
}

fn split_after_folder(path: &Path, marker: &str) -> Option<PathBuf> {
    let comps: Vec<_> = path.components().collect();

    // 找到标记文件夹的位置
    let pos = comps.iter().position(|c| c.as_os_str() == marker)?;

    // 收集该位置之后的所有部分
    Some(comps.iter().skip(pos + 1).collect())
}
