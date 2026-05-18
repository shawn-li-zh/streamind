use axum::{Json, extract::Path};

use entity_service::DB;

use kernel::agent::context::PermissionLife;
use serde::Deserialize;

use crate::wrapper::ApiResponse;

pub async fn tree() -> ApiResponse<Vec<entity::agent::Model>> {
    let agents = entity_service::agent::tree().await;
    match agents {
        Ok(v) => ApiResponse::success(v),
        Err(e) => ApiResponse::error(e),
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct Body {
    model_id: u32,
    name: String,
    avatar: String,
    persona: Option<String>,
}
pub async fn create(Json(body): Json<Body>) -> ApiResponse<u32> {
    let res = entity_service::transaction(|txn| {
        Box::pin(async move {
            let agent = entity_service::agent::save(
                txn,
                body.name,
                body.model_id,
                0,
                body.avatar,
                body.persona.unwrap_or_default(),
                vec!["*".to_string()],
            )
            .await;
            agent
        })
    })
    .await;
    match res {
        Ok(v) => ApiResponse::success(v.id),
        Err(e) => ApiResponse::error(e),
    }
}

pub async fn get_by_id(Path(id): Path<u32>) -> ApiResponse<entity::agent::Model> {
    match entity_service::agent::get_by_id(id).await {
        Ok(v) => {
            if let Some(v) = v {
                ApiResponse::success(v)
            } else {
                ApiResponse::error("Agent不存在")
            }
        }
        Err(e) => ApiResponse::error(e),
    }
}
pub struct PermissionApply {
    pub code: String,
    pub sub_code: Option<String>,
    pub purpose: String,
    pub life: PermissionLife,
}
pub async fn permission_apply(
    Path(agent_id): Path<u32>,
    Json(req): Json<PermissionApply>,
) -> ApiResponse<()> {
    let db = DB.get().unwrap();
    if let Err(err) = entity_service::agent_permission::create(
        db,
        agent_id,
        req.code,
        req.sub_code,
        req.purpose,
        req.life,
    )
    .await
    {
        return ApiResponse::error(err);
    }
    ApiResponse::ok()
}

pub struct RevokeReq {
    pub agent_id: u32,
    pub permission_id: i64,
}

pub async fn permission_revoke(Json(req): Json<RevokeReq>) -> ApiResponse<()> {
    let db = DB.get().unwrap();
    if let Err(err) =
        entity_service::agent_permission::revoke(db, req.agent_id, req.permission_id).await
    {
        return ApiResponse::error(err);
    }
    ApiResponse::ok()
}
