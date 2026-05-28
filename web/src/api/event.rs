use anyhow::Context;
use axum::extract::{Path, Query};

use entity::event::{EnvEventStatus, EnvEventType};

use serde::Deserialize;

use crate::wrapper::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct LoadReq {
    pub agent_id: u32,
    pub event_id: Option<i64>,
    pub size: Option<u64>,
    pub types: Option<String>,
}
pub async fn load(Query(req): Query<LoadReq>) -> ApiResponse<Vec<entity::event::Model>> {
    let types = req.types.map(|ty| {
        ty.split(",")
            .map(|v| {
                serde_json::from_str(&format!("\"{v}\""))
                    .with_context(|| anyhow::anyhow!("parse event type {}", v))
                    .unwrap()
            })
            .collect()
    });
    match entity_service::event::load(req.agent_id, req.event_id, req.size, types).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

pub async fn count_by_status(
    Path(agent_id): Path<u32>,
) -> ApiResponse<Vec<entity_service::event::StatusCount>> {
    let types = vec![
        EnvEventType::UserRequest,
        EnvEventType::Heartbeat,
        EnvEventType::ToolResponse,
        EnvEventType::Error,
        EnvEventType::Timer,
        EnvEventType::Interrupt,
        EnvEventType::Interaction,
    ];
    match entity_service::event::count_by_status(agent_id, Some(types)).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListReq {
    pub agent_id: u32,
    pub status: Option<EnvEventStatus>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

pub async fn list(
    Query(req): Query<ListReq>,
) -> ApiResponse<Vec<entity_service::event::EventItem>> {
    let types = vec![
        EnvEventType::UserRequest,
        EnvEventType::Heartbeat,
        EnvEventType::ToolResponse,
        EnvEventType::Error,
        EnvEventType::Timer,
        EnvEventType::Interrupt,
        EnvEventType::Interaction,
        EnvEventType::Reflection,
    ];
    match entity_service::event::list_env_events(
        req.agent_id,
        Some(types),
        req.status,
        req.page,
        req.size,
    )
    .await
    {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
