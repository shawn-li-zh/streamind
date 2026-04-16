use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use async_openai::{config::OpenAIConfig, Client};
use chrono::Local;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use tokio::sync::RwLock;

use crate::{
    agent::{llm_engine::LLMEngine, Agent, AgentRuntime},
    global::{DB, PROMPT_CACHE},
};
//全局agent池
pub static AGENT_POOL: LazyLock<RwLock<HashMap<u32, Arc<RwLock<Agent>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static AGENT_RUNTIME_POOL: LazyLock<RwLock<HashMap<u32, Arc<RwLock<AgentRuntime>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static ENGINE_POOL: LazyLock<RwLock<HashMap<String, Arc<LLMEngine>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

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
        p.insert("default".to_string(), Arc::new(engine));
    }
    //初始化agent
    let mut agents: Vec<entity::agent::Model> =
        entity::agent::Entity::find().all(DB.get().unwrap()).await?;
    // 如果为空，插入一个默认的agent
    if agents.is_empty() {
        let system = PROMPT_CACHE.get("system").unwrap();
        let agent = entity::agent::ActiveModel {
            id: Set(1),
            name: Set("助手".to_string()),
            state_version: Set(1),
            model: Set(model),
            system: Set(system),
            create_time: Set(Local::now()),
            ..Default::default()
        };
        let agent = agent.insert(DB.get().unwrap()).await?;
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
