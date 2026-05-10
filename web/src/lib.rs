use std::net::SocketAddr;

use axum::{Router, error_handling::HandleErrorLayer, http::HeaderValue, response::IntoResponse};
use reqwest::{Method, header};

use tower_http::cors::CorsLayer;

use crate::wrapper::ApiResponse;

pub mod api;
pub mod feature;
pub mod wrapper;

pub struct Service {
    pub router: Router,
    pub addr: SocketAddr,
}

pub async fn run(service: Service) -> anyhow::Result<()> {
    let router: Router = inject_router(service.router);
    let listener = tokio::net::TcpListener::bind(service.addr).await?;
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn inject_router(mut router: Router) -> Router {
    router = feature::inject_router(router);
    router = api::inject_router(router);
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
    router = router.layer(cors);
    //异常处理
    router = router.layer(HandleErrorLayer::new(|err| async move {
        ApiResponse::<()>::error(&format!("服务器错误: {}", err))
            .code(500)
            .into_response()
    }));
    router
}
