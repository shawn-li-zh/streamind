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
    pub fn get_safe_path(&self, path: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
        // 1. 获取工作区的绝对路径
        let root = Path::new(&self.info.workspace);
        if !root.exists() {
            return Err(anyhow::anyhow!("工作区根目录不存在或无法访问"));
        }
        let path_ = Path::new(path);
        if root.ends_with("/") || path_.starts_with("/") {
            return Ok((
                root.to_path_buf(),
                PathBuf::from(format!("{}{}", &self.info.workspace, path)),
            ));
        }
        Ok((
            root.to_path_buf(),
            PathBuf::from(format!("{}/{}", &self.info.workspace, path)),
        ))
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
    pub workspace: String,
    pub avatar: String,
    pub status: AgentStatus,
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
