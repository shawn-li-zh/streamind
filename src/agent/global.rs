use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use async_openai::{config::OpenAIConfig, Client};
use chrono::Local;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};

use streamind_core::{agent::AgentContext, item::Item, tool::ToolPackagePool};
use tokio::sync::RwLock;

use crate::{
    agent::{
        core::{Agent, AgentRuntime},
        kit::entity_kit,
        llm_engine::LLMEngine,
    },
    global::{DB, PROMPT_CACHE},
};
//全局agent池
pub static AGENT_POOL: LazyLock<RwLock<HashMap<u32, Arc<RwLock<Agent>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static AGENT_CTX_POOL: LazyLock<RwLock<HashMap<u32, Arc<AgentContext>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static ENGINE_POOL: LazyLock<RwLock<HashMap<String, Arc<LLMEngine>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static TOOL_PACKAGE_TOOL: LazyLock<ToolPackagePool> = LazyLock::new(|| tool_pkg::tool_pool());

pub static BASE_ITEMS: &str = include_str!("../../prompts/base_items.json");

pub async fn init() -> anyhow::Result<()> {
    //初始化大模型
    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_base(std::env::var("API_BASE")?)
            .with_api_key(std::env::var("API_KEY")?),
    );
    let model = std::env::var("MODEL")?.to_string();
    let engine = LLMEngine {
        model: model.clone(),
        client,
    };
    {
        let mut p = ENGINE_POOL.write().await;
        p.insert(engine.model.to_string(), Arc::new(engine));
    }
    let database_connection = DB.get().unwrap();
    //初始化agent
    let mut agents: Vec<entity::agent::Model> = entity::agent::Entity::find()
        .all(database_connection)
        .await?;
    // 如果为空，插入一个默认的agent
    if agents.is_empty() {
        let base_items: HashMap<String, Vec<Item>> = serde_json::from_str(BASE_ITEMS)?;
        let agent = entity_kit::new_agent("助手".to_string(), model, 0, base_items).await?;
        agents.push(agent);
    }
    {
        let mut p = AGENT_POOL.write().await;

        for agent_info in agents {
            let id = agent_info.id;
            let agent = Agent::new(agent_info).await;
            p.insert(id, agent);
        }
    }

    Ok(())
}
