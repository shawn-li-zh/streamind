use axum::extract::{Path, Query};

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::{global::DB, web::ApiResponse};
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub agent_id: u32,
}
pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::state::Model>> {
    let db = DB.get().unwrap();

    let states = entity::state::Entity::find()
        .filter(entity::state::Column::AgentId.eq(req.agent_id))
        .filter(entity::state::Column::Dumped.eq(false))
        .order_by_asc(entity::state::Column::Pos)
        .all(db)
        .await
        .unwrap();

    ApiResponse::success(states)
}
