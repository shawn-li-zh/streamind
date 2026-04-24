use std::{
    any::Any,
    collections::HashSet,
    path::PathBuf,
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
        let root = std::env::current_dir()?
            .join(&self.info.workspace)
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("工作区根目录不存在或无法访问"))?;

        // 拼接到根目录
        // 注意：如果是绝对路径（如 "C:\Windows"），join 会直接覆盖掉 root
        let joined: PathBuf = root.join(path);

        //  检查最终路径是否以 root 为前缀
        if joined.starts_with(&root) {
            Ok((root, joined))
        } else {
            Err(anyhow::anyhow!("拒绝访问：路径 {} 超出工作区范围！", path))
        }
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
