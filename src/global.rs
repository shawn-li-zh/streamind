use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};

use crate::{agent::AgentGlobalContext, common::prompt_cache::PromptCache, tool::ToolPackage};
use lazy_static::lazy_static;
use sea_orm::DatabaseConnection;
pub static PROMPT_CACHE: LazyLock<PromptCache> =
    LazyLock::new(|| PromptCache::new("./prompts".into()));
pub static TOOL_PACKAGE_POOL: LazyLock<HashMap<String, ToolPackage>> =
    LazyLock::new(|| HashMap::new());
pub static DB: OnceLock<DatabaseConnection> = OnceLock::new();

lazy_static! {
    pub static ref AGENT_GLOBAL_CONTEXT: AgentGlobalContext = AgentGlobalContext {
        agent: HashMap::new(),
        engine: HashMap::new(),
    };
}
