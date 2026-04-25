use axum::{
    extract::{Path, Query},
    response::sse::Event,
};
use entity::event::{self, EnvEventType};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{global::DB, web::ApiResponse};
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
    pub step: i64,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::section::Model>> {
    let db = DB.get().unwrap();

    let sections = entity::section::Entity::find()
        .filter(entity::section::Column::AgentId.eq(req.agent_id))
        .filter(entity::section::Column::Step.eq(req.step))
        .all(db)
        .await
        .unwrap();

    ApiResponse::success(sections)
}
