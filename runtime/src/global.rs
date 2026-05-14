use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use async_openai::{Client, config::OpenAIConfig};

use entity::model::ModelStatus;

use kernel::{agent::AgentContext, tool::ToolPackagePool};
use tokio::sync::RwLock;

use crate::{core::Agent, llm::{client::ChatClient, deepseek::DeepseekClient}};
//全局agent池
pub static AGENT_POOL: LazyLock<RwLock<HashMap<u32, Arc<RwLock<Agent>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static AGENT_CTX_POOL: LazyLock<RwLock<HashMap<u32, Arc<AgentContext>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static ENGINE_POOL: LazyLock<RwLock<HashMap<u32, Arc<dyn ChatClient + Sync + Send>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static TOOL_PACKAGE_TOOL: LazyLock<ToolPackagePool> = LazyLock::new(|| tool_pkg::tool_pool());

pub async fn init() -> anyhow::Result<()> {
    // 初始化引擎
    let models = entity_service::model::query(None, None, Some(ModelStatus::Active)).await?;
    let model_ids = models.iter().map(|m| m.id).collect::<Vec<u32>>();
    for model in models {
        let mut p = ENGINE_POOL.write().await;
        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_base(model.api_url.clone())
                .with_api_key(model.api_key.clone()),
        );
        let engine = DeepseekClient {
            model: model.clone(),
            client,
        };
        p.insert(model.id, Arc::new(engine));
    }
    //初始化agent
    let agents = entity_service::agent::list_by_model_ids(model_ids).await?;

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
