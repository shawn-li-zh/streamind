use axum::extract::{Path, Query};

use serde::Deserialize;

use crate::wrapper::ApiResponse;
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::state::Model>> {
    match entity_service::state::list(req.agent_id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
