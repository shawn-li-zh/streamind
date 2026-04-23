use axum::{
    extract::{Path, Query},
    response::sse::Event,
};
use entity::event::{self, EventTypeName};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::{global::DB, web::ApiResponse};
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
    pub state_version: i64,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();

    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(req.agent_id))
        .filter(entity::event::Column::StateVersion.eq(req.state_version))
        .order_by_asc(entity::event::Column::Step)
        .all(db)
        .await
        .unwrap();
    ApiResponse::success(events)
}
