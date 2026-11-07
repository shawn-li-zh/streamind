use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query},
};

use entity::format::date_time;
use entity_service::DB;
use serde::{Deserialize, Serialize};

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

pub async fn update_value(
    Path(id): Path<i64>,
    Json(value): Json<serde_json::Value>,
) -> ApiResponse<()> {
    let db = DB.get().unwrap();
    match entity_service::section::update_value(db, id, value).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
#[derive(Debug, Deserialize)]
pub struct ListInRangeReq {
    pub agent_id: u32,
    pub start: i64,
    pub end: i64,
    pub names: Option<String>,
}

pub async fn list_in_range(
    Query(req): Query<ListInRangeReq>,
) -> ApiResponse<Vec<entity_service::section::ListInRangeResp>> {
    let names = req
        .names
        .map(|names| names.split(",").map(|v| v.to_string()).collect());
    match entity_service::section::list_in_range(req.agent_id, req.start, req.end, names).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
