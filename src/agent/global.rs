use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use async_openai::{config::OpenAIConfig, Client};
use chrono::Local;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    agent::{
        core::{Agent, AgentRuntime},
        llm_engine::LLMEngine,
    },
    global::{DB, PROMPT_CACHE},
};
//全局agent池
pub static AGENT_POOL: LazyLock<RwLock<HashMap<u32, Arc<RwLock<Agent>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static AGENT_RUNTIME_POOL: LazyLock<RwLock<HashMap<u32, Arc<RwLock<AgentRuntime>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static ENGINE_POOL: LazyLock<RwLock<HashMap<String, Arc<LLMEngine>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static BASE_ITEMS: &str = include_str!("../../prompts/base_items.json");
#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub content: String,
    pub editable: bool,
}
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
    let database_connection = DB.get().unwrap();
    //初始化agent
    let mut agents: Vec<entity::agent::Model> = entity::agent::Entity::find()
        .all(database_connection)
        .await?;
    // 如果为空，插入一个默认的agent
    if agents.is_empty() {
        let agent = entity::agent::ActiveModel {
            id: Set(1),
            name: Set("助手".to_string()),
            step: Set(0),
            state_version: Set(0),
            model: Set(model),
            create_time: Set(Local::now()),
            ..Default::default()
        };
        let agent = agent.insert(database_connection).await?;
        let base_items: HashMap<String, Vec<Item>> = serde_json::from_str(BASE_ITEMS).unwrap();
        let mut saved_items = vec![];
        for (group, items) in base_items {
            for item in items {
                let model = entity::item::ActiveModel {
                    agent_id: Set(agent.id),
                    group: Set(group.clone()),
                    name: Set(item.name),
                    content: Set(item.content),
                    editable: Set(item.editable),
                    ..Default::default()
                };
                saved_items.push(model);
            }
        }
        entity::item::Entity::insert_many(saved_items)
            .exec(database_connection)
            .await?;
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
