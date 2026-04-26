use std::{
    any::Any,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicI64, AtomicPtr},
    },
};

use crate::{item::ItemGroup, tool::engine::ToolEngineContext};

pub struct AgentContext {
    pub info: AgentInfo,
    // pub item_groups: Vec<ItemGroup>,
    pub tool_engine_context: Arc<ToolEngineContext>,
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
    pub parent_id: u32,
    pub step: i64,
    pub state_id: i64,
    // pub state_pos: u32,
    pub name: String,
    pub model: String,
    pub workspace: PathBuf,
    pub avatar: String,
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
