use axum::{
    Json,
    extract::{Path, Query},
};
use serde::Deserialize;

use crate::wrapper::ApiResponse;
#[derive(Debug, Deserialize)]
pub struct ListReq {
    pub key: Option<String>,
}
pub async fn list(
    Query(req): Query<ListReq>,
) -> ApiResponse<Vec<entity::render_dependence::Model>> {
    match entity_service::dependence::list(req.key).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct Body {
    pub name: String,
    pub version: String,
    pub path: String,
}

pub async fn create(Json(body): Json<Body>) -> ApiResponse<i64> {
    match entity_service::dependence::create(body.name, body.version, body.path).await {
        Ok(v) => ApiResponse::success(v.id),
        Err(e) => ApiResponse::error(e),
    }
}
pub async fn update(Path(id): Path<i64>, Json(body): Json<Body>) -> ApiResponse<()> {
    match entity_service::dependence::update(id, body.name, body.version, body.path).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
pub async fn remove(Path(id): Path<i32>) -> ApiResponse<()> {
    match entity_service::dependence::remove(id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
pub async fn download_to_local(Path(id): Path<i32>) -> ApiResponse<()> {
    match entity_service::dependence::download_to_local(id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
