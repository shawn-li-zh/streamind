use axum::{
    Json,
    extract::{Path, Query},
};
use serde::{Deserialize, Serialize};

use crate::wrapper::ApiResponse;

pub async fn get_by_id(
    Path(id): Path<i64>,
) -> ApiResponse<Option<entity::render_component::Model>> {
    match entity_service::component::get_by_id(id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PageQuery {
    pub page: u64,
    pub size: u64,
    pub key: Option<String>,
}
pub async fn page(
    Query(query): Query<PageQuery>,
) -> ApiResponse<Vec<entity::render_component::Model>> {
    match entity_service::component::page(query.page, query.size, query.key).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TotalQuery {
    pub key: Option<String>,
}
pub async fn total(Query(query): Query<TotalQuery>) -> ApiResponse<u64> {
    match entity_service::component::total(query.key).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

pub async fn create(Json(body): Json<entity_service::component::Body>) -> ApiResponse<i64> {
    match entity_service::component::create(body).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

pub async fn update(
    Path(id): Path<i64>,
    Json(body): Json<entity_service::component::Body>,
) -> ApiResponse<()> {
    match entity_service::component::update(id, body).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
