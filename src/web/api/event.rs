use axum::extract::{Path, Query};

use entity::event::EnvEventType;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;

use crate::{agent::global::AGENT_POOL, global::DB, web::ApiResponse};
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
    pub state_id: i64,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();

    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(req.agent_id))
        .filter(entity::event::Column::StateId.eq(req.state_id))
        .order_by_asc(entity::event::Column::Step)
        .all(db)
        .await
        .unwrap();
    ApiResponse::success(events)
}

#[derive(Debug, Deserialize)]
pub struct LoadReq {
    pub agent_id: u32,
    pub event_id: Option<i64>,
    pub size: Option<u64>,
    pub types: Option<Vec<EnvEventType>>,
}
pub async fn load(Query(req): Query<LoadReq>) -> ApiResponse<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();
    let mut select =
        entity::event::Entity::find().filter(entity::event::Column::AgentId.eq(req.agent_id));
    if let Some(event_id) = req.event_id {
        select = select.filter(entity::event::Column::Id.lt(event_id))
    }
    if let Some(types) = req.types {
        select = select.filter(entity::event::Column::EventType.is_in(types));
    }
    let size = req.size.unwrap_or(30);
    let events = select
        .order_by_asc(entity::event::Column::Id)
        .limit(size)
        .all(db)
        .await
        .unwrap();
    ApiResponse::success(events)
}
