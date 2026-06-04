use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, OnceLock},
};

use async_openai::{config::OpenAIConfig, Client};

use entity::model::ModelStatus;

use kernel::{
    client::ChatClient,
    embedding::{self, EmbeddingClient},
    model::ModelModal,
    tool::ToolPackagePool,
};

use parking_lot::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};

use runtime::{
    embedding::doubao::DoubaoEmbedding,
    llm::{deepseek::DeepseekClient, openai::OpenaiClient, siliconflow::SiliconflowClient},
};

use crate::{agent::app::AppAgent, task};
//全局agent池
pub static AGENT_POOL: LazyLock<RwLock<HashMap<u32, Arc<AppAgent>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static ENGINE_POOL: LazyLock<RwLock<HashMap<u32, Arc<dyn ChatClient + Sync + Send>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static TOOL_PACKAGE_TOOL: LazyLock<ToolPackagePool> = LazyLock::new(|| tool_pkg::tool_pool());
pub(crate) static SCHEDULER: OnceLock<JobScheduler> = OnceLock::new();
pub static EMBEDDING_CLIENT: OnceLock<Arc<dyn EmbeddingClient>> = OnceLock::new();
pub async fn init() -> anyhow::Result<()> {
    // 初始化引擎
    let models = entity_service::model::query(None, None, Some(ModelStatus::Active)).await?;
    let model_ids = models.iter().map(|m| m.id).collect::<Vec<u32>>();
    for model in models {
        let mut p = ENGINE_POOL.write();
        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_base(model.api_url.clone())
                .with_api_key(model.api_key.clone()),
        );
        let value = serde_json::to_value(model.input_modals.0)?;
        let modals = serde_json::from_value::<Vec<ModelModal>>(value)?;

        let engine = SiliconflowClient {
            model: model.name.clone(),
            client,
            modals: modals,
        };
        p.insert(model.id, Arc::new(engine));
    }
    //初始化嵌入模型
    {
        let api_key = std::env::var("EMBEDDING_API_KEY")?;
        let api_url = std::env::var("EMBEDDING_API_URL")?;
        let model = std::env::var("EMBEDDING_MODEL")?;
        let dim = std::env::var("EMBEDDING_DIM")
            .map(|v| v.parse::<u32>().map_err(|e| anyhow::anyhow!(e)))
            .map_err(|e| anyhow::anyhow!(e))
            .flatten()?;
        let embedding = DoubaoEmbedding::new(api_key, api_url, model, dim);
        if let Err(_) = EMBEDDING_CLIENT.set(Arc::new(embedding)) {
            anyhow::bail!("初始化向量模型失败");
        }
    }
    //初始化定时调度器
    {
        let scheduler: JobScheduler = JobScheduler::new().await?;
        if let Err(_) = SCHEDULER.set(scheduler) {
            anyhow::bail!("初始化定时调度器失败");
        }
        task::init().await;
        SCHEDULER.get().unwrap().start().await?;
    }

    //初始化agent
    let agents = entity_service::agent::list_by_model_ids(model_ids).await?;
    {
        let mut p = AGENT_POOL.write();
        for agent_info in agents {
            let id = agent_info.id;
            let (agent, runtime) = AppAgent::new(agent_info).await;
            runtime.start();
            // let agent = Arc::new(agent);
            p.insert(id, agent);
        }
    }

    Ok(())
}
