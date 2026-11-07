use axum::Router;

use tracing::info;
use tracing_subscriber::EnvFilter;
use web_api::feature::auth::LOGIN_CODE;

mod common;
mod event_context;
mod global;
mod plugin;
mod runtime;
mod sse_event;
mod status;
mod agent_api;
mod agent;
mod task;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    //
    entity_service::init().await?;
    info!("存储服务初始化成功!");
    // runtime::global::init().await?;
    global::init().await?;
    info!("智能体运行时初始化成功!");
    //
    let port = &std::env::var("PORT").unwrap();
    let web_service = web_api::Service {
        router: agent_api::inject_router(Router::new()),
        addr: format!("0.0.0.0:{port}").parse().unwrap(),
    };
    info!("Starting StreamInd AI Client");
    info!("Generated login code:\n 【 {} 】", LOGIN_CODE.as_str());
    web_api::run(web_service).await?;

    Ok(())
}
