use axum::extract::{Path, Query};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{global::DB, web::ApiResponse};
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
    pub event_id: i64,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::section::Model>> {
    let db = DB.get().unwrap();

    let sections = entity::section::Entity::find()
        .filter(entity::section::Column::AgentId.eq(req.agent_id))
        .filter(entity::section::Column::EventId.eq(req.event_id))
        .all(db)
        .await
        .unwrap();

    ApiResponse::success(sections)
}

pub async fn get_by_id(Path(req): Path<i64>) -> ApiResponse<Option<entity::section::Model>> {
    let db = DB.get().unwrap();

    let section = entity::section::Entity::find()
        .filter(entity::section::Column::Id.eq(req))
        .one(db)
        .await
        .unwrap();

    ApiResponse::success(section)
}
