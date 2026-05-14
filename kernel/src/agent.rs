use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::Local;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{
    event::EnvEvent,
    message::{Message, Role},
    tool::engine::ToolEngineContext,
};

pub struct AgentContext {
    pub info: AgentInfo,
    // pub item_groups: Vec<ItemGroup>,
    pub tool_engine_context: Arc<ToolEngineContext>,
    pub permissions: DashMap<String, AgentPermission>,
}

pub struct AgentPermission {
    pub code: String,
    pub sub_code: Option<String>,
    pub life: PermissionLife,
    pub use_times: u32,
    pub create_time: chrono::DateTime<chrono::Local>,
}
impl AgentPermission {
    pub fn is_valid(&self) -> bool {
        match self.life {
            PermissionLife::None => false,
            PermissionLife::Once => self.use_times == 0,
            PermissionLife::Times(times) => self.use_times < times,
            PermissionLife::Duration(duration) => {
                let expire_time = self.create_time + chrono::Duration::from_std(duration).unwrap();
                chrono::Local::now() < expire_time
            }
            PermissionLife::Forever => true,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum PermissionLife {
    #[default]
    None,
    Once,
    Times(u32),
    Duration(std::time::Duration),
    Forever,
}

impl AgentContext {
    pub fn get_real_path(&self, path: &str) -> anyhow::Result<(PathBuf, bool)> {
        self.info.get_real_path(path)
    }
    pub fn to_short_path(&self, path: &str) -> anyhow::Result<PathBuf> {
        self.info.to_short_path(path)
    }
}
pub struct AgentInfo {
    pub id: u32,
    pub code: String,
    pub model_id: u32,
    pub parent_id: u32,
    pub state_id: i64,
    // pub state_pos: u32,
    pub name: String,
    pub model: String,
    pub workspace: PathBuf,
    pub avatar: String,
    pub sleep_until: Option<chrono::DateTime<Local>>,
    pub status: AgentStatus,
}
impl AgentInfo {
    pub fn get_real_path(&self, path: &str) -> anyhow::Result<(PathBuf, bool)> {
        // 1. 获取工作区的绝对路径
        let root = Path::new(&self.workspace);
        if !root.exists() {
            return Err(anyhow::anyhow!("工作区根目录不存在或无法访问"));
        }
        let real_path = root.join(path);
        let is_safe = real_path.starts_with(root);
        Ok((real_path, is_safe))
    }

    pub fn to_short_path(&self, path: &str) -> anyhow::Result<PathBuf> {
        // 1. 获取工作区的绝对路径
        let root = Path::new(&self.workspace);
        if !root.exists() {
            return Err(anyhow::anyhow!("工作区根目录不存在或无法访问"));
        }
        let path = Path::new(path);
        if let Ok(path) = path.strip_prefix(root) {
            return Ok(path.to_path_buf());
        }
        Ok(path.to_path_buf())
    }
}

#[derive(Debug)]
pub enum AgentStatus {
    //输出中
    Running,
    //空闲
    Idle,
    //整理记忆
    Organizing,
    //离线
    Offline,
}

//事件
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub id: i64,
    pub data: EnvEvent,
    //是否强制执行
    pub force: bool,
    pub time: chrono::DateTime<Local>,
}
impl AgentEvent {
    pub fn to_infos(&self) -> Vec<crate::message::Information> {
        let time = self.time.format("%Y-%m-%d %H:%M:%S").to_string();
        let head = format!(
            "# 事件\nid={},type={},time={}",
            self.id,
            self.data.name(),
            time
        );
        let mut infos = self.data.clone().to_infos();
        infos.insert(0, head.into());
        return infos;
    }
    pub fn to_message(&self) -> Message {
        let msg = match &self.data {
            EnvEvent::AgentReaction(sections) | EnvEvent::AgentResponse(sections) => {
                Role::assistant()
            }
            EnvEvent::UserRequest(user_message) => Role::user(),
            EnvEvent::ToolCallResult(tool_call_detail_items) => Role::system(),
            _ => Role::system(),
        };
        let infos = self.to_infos();
        msg.infos(infos)
    }
}
