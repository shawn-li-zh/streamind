use axum::{routing::get, Router};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::{agent::global::AGENT_POOL, web::inject_router};

mod agent;
mod common;
mod global;
mod rule;
mod tool;
mod web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    dotenvy::dotenv()?;
    crate::global::init().await?;
    agent::global::init().await?;
    info!("Starting StreamInd AI Client");

    let mut app = Router::new();
    app = inject_router(app);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
