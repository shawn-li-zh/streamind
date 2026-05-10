use axum::extract::{Path, Query};

use entity::event::EnvEventType;

use serde::Deserialize;

use crate::wrapper::ApiResponse;
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
    pub state_id: i64,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::event::Model>> {
    match entity_service::event::list(req.agent_id, req.state_id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct LoadReq {
    pub agent_id: u32,
    pub event_id: Option<i64>,
    pub size: Option<u64>,
    pub types: Option<Vec<EnvEventType>>,
}
pub async fn load(Query(req): Query<LoadReq>) -> ApiResponse<Vec<entity::event::Model>> {
    match entity_service::event::load(req.agent_id, req.event_id, req.size, req.types).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
