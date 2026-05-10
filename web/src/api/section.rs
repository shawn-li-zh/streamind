use axum::extract::{Path, Query};

use serde::Deserialize;

use crate::wrapper::ApiResponse;
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
    pub event_id: i64,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::section::Model>> {
    match entity_service::section::list(req.agent_id, req.event_id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

pub async fn get_by_id(Path(id): Path<i64>) -> ApiResponse<Option<entity::section::Model>> {
    match entity_service::section::get_by_id(id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
