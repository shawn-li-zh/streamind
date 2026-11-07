use axum::{
    Json,
    extract::{Path, Query},
};
use entity::model::{ModelCost, ModelStatus};

use entity_service::DB;
use serde::Deserialize;

use crate::wrapper::ApiResponse;

pub async fn create(Json(body): Json<entity_service::model::Form>) -> ApiResponse<u32> {
    let db = DB.get().unwrap();
    match entity_service::model::save(db, body).await {
        Ok(v) => ApiResponse::success(v.id),
        Err(e) => ApiResponse::error(e),
    }
}

pub async fn update(
    Path(id): Path<u32>,
    Json(body): Json<entity_service::model::Form>,
) -> ApiResponse<()> {
    let db = DB.get().unwrap();
    match entity_service::model::update(db, id, body).await {
        Ok(_) => ApiResponse::ok(),
        Err(e) => ApiResponse::error(e),
    }
}
#[derive(Debug, Deserialize)]
pub struct QueryReq {
    pub name: Option<String>,
    pub modal: Option<String>,
    pub status: Option<ModelStatus>,
}

pub async fn list(Query(req): Query<QueryReq>) -> ApiResponse<Vec<entity::model::Model>> {
    match entity_service::model::query(req.name, req.modal, req.status).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
