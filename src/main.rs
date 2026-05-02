use axum::{http::HeaderValue, routing::get, Router};
use reqwest::{header, Method};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::{agent::global::AGENT_POOL, web::inject_router};

mod agent;
mod common;
mod global;
mod rule;

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
    //映射本地资源
    let serve_dir = tower_http::services::ServeDir::new("./public")
        .precompressed_gzip()
        .append_index_html_on_directories(true);
    app = app.nest_service("/public", serve_dir);
    // 定义 CORS 策略
    let cors = CorsLayer::new()
        // 1. 必须指定明确的源，不能用 Any
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        // 2. 必须显式允许这些方法
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS, // 预检请求需要 OPTIONS
        ])
        // 3. 必须显式允许请求头（如 Content-Type, Authorization）
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
        // 4. 解决你报错的关键：允许携带凭据
        .allow_credentials(true);
    app = app.layer(cors);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
