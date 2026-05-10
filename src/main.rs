use axum::Router;

use tracing::info;
use tracing_subscriber::EnvFilter;
use web::feature::auth::LOGIN_CODE;

// mod agent;
mod common;
mod global;
mod rule;
mod web_api;

// mod web;

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
    runtime::global::init().await?;
    info!("智能体运行时初始化成功!");
    //
    let web_service = web::Service {
        router: web_api::inject_router(Router::new()),
        addr: "0.0.0.0:3000".parse().unwrap(),
    };
    info!("Starting StreamInd AI Client");
    info!("Generated login code:\n 【 {} 】", LOGIN_CODE.as_str());
    web::run(web_service).await?;

    Ok(())
}
