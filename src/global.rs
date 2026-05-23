use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, OnceLock},
};

use async_openai::{config::OpenAIConfig, Client};

use entity::model::ModelStatus;

use kernel::{client::ChatClient, tool::ToolPackagePool};

use parking_lot::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};

use runtime::llm::deepseek::DeepseekClient;

use crate::agent::AppAgent;
//全局agent池
pub static AGENT_POOL: LazyLock<RwLock<HashMap<u32, Arc<AppAgent>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static ENGINE_POOL: LazyLock<RwLock<HashMap<u32, Arc<dyn ChatClient + Sync + Send>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static TOOL_PACKAGE_TOOL: LazyLock<ToolPackagePool> = LazyLock::new(|| tool_pkg::tool_pool());
pub(crate) static SCHEDULER: OnceLock<JobScheduler> = OnceLock::new();

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
        let engine = DeepseekClient {
            model: model.name.clone(),
            client,
        };
        p.insert(model.id, Arc::new(engine));
    }
    //初始化定时调度器
    {
        let scheduler: JobScheduler = JobScheduler::new().await?;
        if let Err(_) = SCHEDULER.set(scheduler) {
            anyhow::bail!("初始化定时调度器失败");
        }
    }
    //初始化agent
    let agents = entity_service::agent::list_by_model_ids(model_ids).await?;

    {
        let mut p = AGENT_POOL.write();
        for agent_info in agents {
            let id = agent_info.id;
            let (agent, runtime) = AppAgent::new(agent_info).await;
            runtime.start();
            // agent.scheduler.clone().start()?;
            // runtime.start(mind);
            // let agent = Arc::new(agent);
            p.insert(id, agent);
        }
    }

    Ok(())
}
