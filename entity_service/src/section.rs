use std::collections::HashMap;

use kernel::mind::section::Section;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde::Serialize;

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
        name: Set(section.name),
        content: Set(section.content),
        elapsed: Set(section.elapsed),
        payload: Set(payload),
        value: Set(serde_json::Value::Null),
        ..Default::default()
    };
    let model = model.insert(db).await?;
    Ok(model)
}

pub async fn update_value<T: Serialize>(
    db: &impl ConnectionTrait,
    id: i64,
    value: T,
) -> anyhow::Result<()> {
    let model = entity::section::ActiveModel {
        value: Set(serde_json::to_value(value)?),
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
        .await?;

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
        .await?;

    Ok(sections)
}
pub async fn get_by_id(id: i64) -> anyhow::Result<Option<entity::section::Model>> {
    let db = DB.get().unwrap();

    let section = entity::section::Entity::find()
        .filter(entity::section::Column::Id.eq(id))
        .one(db)
        .await?;

    Ok(section)
}
#[derive(Debug, Serialize)]
pub struct ListInRangeResp {
    pub event: entity::event::Model,
    pub sections: Vec<entity::section::Model>,
}
pub async fn list_in_range(
    agent_id: u32,
    start: i64,
    end: i64,
    names: Option<Vec<String>>,
) -> anyhow::Result<Vec<ListInRangeResp>> {
    let db = DB.get().unwrap();
    let mut select = entity::section::Entity::find()
        .filter(entity::section::Column::AgentId.eq(agent_id))
        .filter(entity::section::Column::EventId.between(start, end));
    if let Some(names) = names {
        select = select.filter(entity::section::Column::Name.is_in(&names));
    }

    let section = select.all(db).await?;
    let mut map: HashMap<i64, Vec<entity::section::Model>> = HashMap::new();
    for ele in section {
        map.entry(ele.event_id).or_default().push(ele);
    }
    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::Id.between(start, end))
        .all(db)
        .await?;
    let resp = events
        .into_iter()
        .map(|event| {
            let sections = map.remove(&event.id).unwrap_or_default();
            ListInRangeResp { event, sections }
        })
        .collect();
    Ok(resp)
}
