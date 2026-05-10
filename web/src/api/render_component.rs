use axum::extract::{Path, Query};

use crate::wrapper::ApiResponse;

pub async fn get_by_id(
    Path(id): Path<i64>,
) -> ApiResponse<Option<entity::render_component::Model>> {
    match entity_service::component::get_by_id(id).await {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
