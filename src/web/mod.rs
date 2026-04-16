use axum::{body::Body, http::Response, response::IntoResponse, Json, Router};
use serde::Serialize;

pub mod api;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub data: Option<T>,
    pub message: String,
}

// 构造辅助函数
impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            data: Some(data),
            message: "ok".to_string(),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            code: 500,
            data: None,
            message: message.to_string(),
        }
    }
}
impl ApiResponse<()> {
    pub fn ok() -> ApiResponse<()> {
        ApiResponse {
            code: 200,
            data: None,
            message: "ok".to_string(),
        }
    }
}
impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response<Body> {
        // 将结构体转换为 JSON，Axum 会自动设置 Content-Type: application/json
        Json(self).into_response()
    }
}

pub fn inject_router(router: Router) -> Router {
    api::inject_router(router)
}
