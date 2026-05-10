use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde::Serialize;
use kernel::section::Section;

use crate::DB;

pub async fn save(
    db: &impl ConnectionTrait,
    agent_id: u32,
    event_id: i64,
    section: Section,
    payload: serde_json::Value,
) -> anyhow::Result<entity::section::Model> {
    let model = entity::section::ActiveModel {
        agent_id: Set(agent_id),
        event_id: Set(event_id),
        name: Set(section.tag_name),
        content: Set(section.content),
        payload: Set(payload),
        time: Set(section.time),
        ..Default::default()
    };
    let model = model.insert(db).await?;
    Ok(model)
}

pub async fn update_payload<T: Serialize>(
    db: &impl ConnectionTrait,
    id: i64,
    payload: T,
) -> anyhow::Result<()> {
    let model = entity::section::ActiveModel {
        payload: Set(serde_json::to_value(payload)?),
        ..Default::default()
    };
    let result = entity::section::Entity::update_many()
        .set(model)
        .filter(entity::section::Column::Id.eq(id))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}

pub async fn list(agent_id: u32, event_id: i64) -> anyhow::Result<Vec<entity::section::Model>> {
    let db = DB.get().unwrap();

    let sections = entity::section::Entity::find()
        .filter(entity::section::Column::AgentId.eq(agent_id))
        .filter(entity::section::Column::EventId.eq(event_id))
        .all(db)
        .await
        .unwrap();

    Ok(sections)
}
pub async fn list_by_event_ids(
    agent_id: u32,
    event_ids: Vec<i64>,
) -> anyhow::Result<Vec<entity::section::Model>> {
    let db = DB.get().unwrap();

    let sections = entity::section::Entity::find()
        .filter(entity::section::Column::AgentId.eq(agent_id))
        .filter(entity::section::Column::EventId.is_in(event_ids))
        .all(db)
        .await
        .unwrap();

    Ok(sections)
}
pub async fn get_by_id(id: i64) -> anyhow::Result<Option<entity::section::Model>> {
    let db = DB.get().unwrap();

    let section = entity::section::Entity::find()
        .filter(entity::section::Column::Id.eq(id))
        .one(db)
        .await
        .unwrap();

    Ok(section)
}
