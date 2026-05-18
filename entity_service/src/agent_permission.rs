use chrono::Local;
use kernel::agent::context::PermissionLife;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde_json::json;

use crate::DB;

pub async fn list(agent_id: u32) -> anyhow::Result<Vec<entity::agent_permission::Model>> {
    let db = DB.get().unwrap();
    //获取权限
    let res = entity::agent_permission::Entity::find()
        .filter(entity::agent_permission::Column::AgentId.eq(agent_id))
        .filter(entity::agent_permission::Column::Disused.eq(false))
        .all(db)
        .await?;
    Ok(res)
}
pub async fn get(agent_id: u32, id: i64) -> anyhow::Result<Option<entity::agent_permission::Model>> {
    let db = DB.get().unwrap();
    //获取权限
    let res = entity::agent_permission::Entity::find()
        .filter(entity::agent_permission::Column::AgentId.eq(agent_id))
        .filter(entity::agent_permission::Column::Id.eq(id))
        .filter(entity::agent_permission::Column::Disused.eq(false))
        .one(db)
        .await?;
    Ok(res)
}

pub async fn revoke(db: &impl ConnectionTrait, agent_id: u32, id: i64) -> anyhow::Result<()> {
    entity::agent_permission::Entity::update_many()
        .set(entity::agent_permission::ActiveModel {
            disused: Set(true),
            ..Default::default()
        })
        .filter(entity::agent_permission::Column::AgentId.eq(agent_id))
        .filter(entity::agent_permission::Column::Id.eq(id))
        .filter(entity::agent_permission::Column::Disused.eq(false))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn create(
    db: &impl ConnectionTrait,
    agent_id: u32,
    code: String,
    sub_code: Option<String>,
    purpose: String,
    life: PermissionLife,
) -> anyhow::Result<i64> {
    let mut query = entity::agent_permission::Entity::find()
        .filter(entity::agent_permission::Column::AgentId.eq(agent_id))
        .filter(entity::agent_permission::Column::Code.eq(&code))
        .filter(entity::agent_permission::Column::Disused.eq(false));
    if let Some(sub_code) = &sub_code {
        query = query.filter(entity::agent_permission::Column::SubCode.eq(sub_code))
    }
    let one = query.one(db).await.unwrap();
    if let Some(one) = one {
        if !one.disused {
            return Err(anyhow::anyhow!("权限已存在"));
        }
    }
    let model = entity::agent_permission::ActiveModel {
        agent_id: Set(agent_id),
        code: Set(code),
        sub_code: Set(sub_code),
        purpose: Set(purpose),
        life: Set(json!(life)),
        use_times: Set(0),
        create_time: Set(Local::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(model.id)
}
