use axum::{
    body::Body,
    extract::{Path, Query},
    response::sse::Event,
    Json,
};
use entity::event::{self, EnvEventType};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{global::DB, web::ApiResponse};

pub async fn get_by_id(Path(id): Path<i64>) -> ApiResponse<entity::render_state::Model> {
    let db = DB.get().unwrap();
    let render_state = entity::render_state::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap();
    if render_state.is_none() {
        return ApiResponse::error("组件状态不存在");
    }
    ApiResponse::success(render_state.unwrap())
}
#[derive(Debug, Deserialize)]
pub struct PosReq {
    pub response_event_id: i64,
    pub section_index: u32,
}
pub async fn get_by_pos(Query(req): Query<PosReq>) -> ApiResponse<entity::render_state::Model> {
    let db = DB.get().unwrap();
    let render_state = entity::render_state::Entity::find()
        .filter(entity::render_state::Column::ResponseEventId.eq(req.response_event_id))
        .filter(entity::render_state::Column::SectionIndex.eq(req.section_index))
        .one(db)
        .await
        .unwrap();
    if render_state.is_none() {
        return ApiResponse::error("组件状态不存在");
    }
    ApiResponse::success(render_state.unwrap())
}

pub async fn set_state_by_id(
    Path(id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> ApiResponse<()> {
    let db = DB.get().unwrap();
    entity::render_state::Entity::update_many()
        .col_expr(
            entity::render_state::Column::State,
            sea_orm::sea_query::Expr::value(body.clone()),
        )
        .filter(entity::render_state::Column::Id.eq(id))
        .exec(db)
        .await
        .unwrap();

    ApiResponse::success(())
}
