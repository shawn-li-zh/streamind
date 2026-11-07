use axum::{Json, body::Body, response::{IntoResponse, Response}};
use reqwest::StatusCode;
use serde::Serialize;

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

    // pub fn error(message: &str) -> Self {
    //     Self {
    //         code: 500,
    //         data: None,
    //         message: message.to_string(),
    //     }
    // }
    pub fn code(mut self, code: u16) -> Self {
        self.code = code;
        self
    }
    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }
    pub fn error<E: std::fmt::Display>(err: E) -> Self {
        Self {
            code: 500,
            data: None,
            message: err.to_string(),
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
        (
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(self),
        )
            .into_response()
    }
}
