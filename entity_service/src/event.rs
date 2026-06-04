use chrono::Local;
use entity::{
    agent::AgentStatusEnum,
    event::{EnvEventStatus, EnvEventType},
    format::date_time,
    state,
};

use kernel::mind::section::Section;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityOrSelect, EntityTrait,
    FromJsonQueryResult, FromQueryResult, QueryFilter, QueryOrder, QuerySelect,
};
use serde::Serialize;

use crate::DB;

pub async fn get_by_id(agent_id: u32, id: i64) -> anyhow::Result<Option<entity::event::Model>> {
    let db = DB.get().unwrap();
    let model = entity::event::Entity::find_by_id(id)
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .one(db)
        .await?;
    Ok(model)
}
pub async fn list_by_ids(
    agent_id: u32,
    ids: Vec<i64>,
) -> anyhow::Result<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();
    let models = entity::event::Entity::find()
        .filter(entity::event::Column::Id.is_in(ids))
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .all(db)
        .await?;
    Ok(models)
}
pub async fn time_range(
    agent_id: u32,
    start: chrono::DateTime<Local>,
    end: chrono::DateTime<Local>,
    event_type: Option<String>,
    mut key: Option<String>,
    limit: u64,
) -> anyhow::Result<Vec<entity::event::Model>> {
    key = key.filter(|s| !s.is_empty());
    let db = DB.get().unwrap();
    let mut query = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::Time.between(start, end));
    if let Some(key) = key {
        query = query.filter(entity::event::Column::Data.like(key));
    }
    if let Some(event_type) = event_type {
        query = query.filter(entity::event::Column::EventType.eq(event_type));
    }
    let models = query
        .order_by_desc(entity::event::Column::Time)
        .limit(limit)
        .all(db)
        .await?;
    Ok(models)
}

pub async fn search_at(
    agent_id: u32,
    id: i64,
    event_type: Option<String>,
    mut key: Option<String>,
    limit: u64,
    is_before: bool,
) -> anyhow::Result<Vec<entity::event::Model>> {
    key = key.filter(|s| !s.is_empty());
    let db = DB.get().unwrap();
    let mut query =
        entity::event::Entity::find().filter(entity::event::Column::AgentId.eq(agent_id));
    if is_before {
        query = query.filter(entity::event::Column::Id.lt(id));
    } else {
        query = query.filter(entity::event::Column::Id.gt(id));
    }
    if let Some(key) = key {
        query = query.filter(entity::event::Column::Data.like(key));
    }
    if let Some(event_type) = event_type {
        query = query.filter(entity::event::Column::EventType.eq(event_type));
    }
    let models = query
        .order_by_desc(entity::event::Column::Time)
        .limit(limit)
        .all(db)
        .await?;
    Ok(models)
}

pub async fn save(
    db: &impl ConnectionTrait,
    agent_id: u32,
    state_id: i64,
    event_type: EnvEventType,
    event: serde_json::Value,
    force: bool,
    size: usize,
) -> anyhow::Result<entity::event::Model> {
    let event_model = entity::event::ActiveModel {
        agent_id: Set(agent_id),
        state_id: Set(state_id),
        step: Set(0),
        event_type: Set(event_type),
        force: Set(force),
        time: Set(Local::now()),
        data: Set(event),
        status: Set(EnvEventStatus::Pending),
        size: Set(size as _),
        ..Default::default()
    };
    let model: entity::event::Model = event_model.insert(db).await?;
    Ok(model)
}

pub async fn update_agent_response(
    db: &impl ConnectionTrait,
    event_id: i64,
    sections: Vec<Section>,
) -> anyhow::Result<()> {
    if sections.is_empty() {
        //删除
        let result = entity::event::Entity::delete_many()
            .filter(entity::event::Column::Id.eq(event_id))
            .filter(entity::event::Column::EventType.eq(EnvEventType::AgentResponse))
            .exec(db)
            .await?;
        return if result.rows_affected == 1 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("删除失败:{}", event_id))
        };
    }
    let data = serde_json::to_value(sections)?;
    let model = entity::event::ActiveModel {
        data: Set(data),
        status: Set(EnvEventStatus::Done),
        ..Default::default()
    };
    let result = entity::event::Entity::update_many()
        .set(model)
        .filter(entity::event::Column::Id.eq(event_id))
        .filter(entity::event::Column::EventType.eq(EnvEventType::AgentResponse))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}
pub async fn unload_events(db: &impl ConnectionTrait, state_id: i64) -> anyhow::Result<()> {
    let model = entity::event::ActiveModel {
        status: Set(EnvEventStatus::Unloaded),
        ..Default::default()
    };
    entity::event::Entity::update_many()
        .filter(entity::event::Column::StateId.eq(state_id))
        .set(model)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn processing(db: &impl ConnectionTrait, agent_id: u32) -> anyhow::Result<()> {
    let model = entity::event::ActiveModel {
        status: Set(EnvEventStatus::Processing),
        ..Default::default()
    };
    entity::event::Entity::update_many()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::Status.eq(EnvEventStatus::Pending))
        .set(model)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn done(db: &impl ConnectionTrait, agent_id: u32) -> anyhow::Result<()> {
    let model = entity::event::ActiveModel {
        status: Set(EnvEventStatus::Done),
        ..Default::default()
    };
    entity::event::Entity::update_many()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::Status.eq(EnvEventStatus::Processing))
        .set(model)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn list(agent_id: u32, state_id: i64) -> anyhow::Result<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();
    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::StateId.eq(state_id))
        .order_by_asc(entity::event::Column::Step)
        .all(db)
        .await?;
    Ok(events)
}

pub async fn list_by_state_ids(
    agent_id: u32,
    state_ids: Vec<i64>,
) -> anyhow::Result<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();
    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::StateId.is_in(state_ids))
        .filter(entity::event::Column::Status.ne(EnvEventStatus::Unloaded))
        .order_by_asc(entity::event::Column::Step)
        .all(db)
        .await?;
    Ok(events)
}
pub async fn load(
    agent_id: u32,
    event_id: Option<i64>,
    size: Option<u64>,
    types: Option<Vec<EnvEventType>>,
) -> anyhow::Result<Vec<entity::event::Model>> {
    let db = DB.get().unwrap();
    let mut select =
        entity::event::Entity::find().filter(entity::event::Column::AgentId.eq(agent_id));
    if let Some(event_id) = event_id {
        select = select.filter(entity::event::Column::Id.lt(event_id))
    }
    if let Some(types) = types {
        select = select.filter(entity::event::Column::EventType.is_in(types));
    }
    //往前加载30条数据
    let size = size.unwrap_or(30);
    let mut events = select
        .order_by_desc(entity::event::Column::Id)
        .limit(size)
        .all(db)
        .await?;
    // 变成正序
    events.reverse();
    Ok(events)
}
#[derive(Debug, Serialize, FromQueryResult)]
pub struct EventItem {
    pub id: i64,
    pub event_type: EnvEventType,

    pub status: EnvEventStatus,
    pub force: bool,
    #[serde(with = "date_time")]
    pub time: chrono::DateTime<chrono::Local>,
}
pub async fn list_env_events(
    agent_id: u32,
    types: Option<Vec<EnvEventType>>,
    status: Option<EnvEventStatus>,
    page: Option<u64>,
    size: Option<u64>,
) -> anyhow::Result<Vec<EventItem>> {
    let db = DB.get().unwrap();
    let mut select = entity::event::Entity::find()
        .select_only()
        .column(entity::event::Column::Id)
        .column(entity::event::Column::EventType)
        .column(entity::event::Column::Time)
        .column(entity::event::Column::Status)
        .column(entity::event::Column::Force)
        .filter(entity::event::Column::AgentId.eq(agent_id));
    if let Some(types) = types {
        select = select.filter(entity::event::Column::EventType.is_in(types));
    }
    if let Some(status) = status {
        select = select.filter(entity::event::Column::Status.eq(status));
    }
    let page = page.unwrap_or(1);
    let size = size.unwrap_or(30);
    let events = select
        .order_by_desc(entity::event::Column::Id)
        .offset((page - 1) * size)
        .limit(size)
        .into_model::<EventItem>()
        .all(db)
        .await?;
    Ok(events)
}

#[derive(FromQueryResult, Serialize)]
pub struct StatusCount {
    pub status: EnvEventStatus,
    pub count: i64, // 注意：COUNT(*) 在大多数数据库返回 i64/BigInt
}
//统计各个状态的事件数量
pub async fn count_by_status(
    agent_id: u32,
    types: Option<Vec<EnvEventType>>,
) -> anyhow::Result<Vec<StatusCount>> {
    let db = DB.get().unwrap();
    let mut select = entity::event::Entity::find()
        .select_only() // 重点：不选所有字段，只选我们需要的
        .column(entity::event::Column::Status) // 分组字段
        .column_as(entity::event::Column::Id.count(), "count") // 统计字段 + 别名
        .filter(entity::event::Column::AgentId.eq(agent_id));
    if let Some(types) = types {
        select = select.filter(entity::event::Column::EventType.is_in(types));
    }
    let counts = select
        .group_by(entity::event::Column::Status)
        .into_model::<StatusCount>()
        .all(db)
        .await?;
    Ok(counts)
}

pub mod event_convert {
    use std::collections::HashMap;

    use entity::event::EnvEventType;
    use runtime::agent::event::{AppEnvEvent, AppEnvEventData, UserMessage};
    use serde_json::json;

    use super::*;

    pub fn to_json(event: &AppEnvEventData) -> serde_json::Value {
        match event {
            AppEnvEventData::AgentResponse(sections) => json!(sections),
            AppEnvEventData::Reflection(event) => json!(event),
            AppEnvEventData::Heartbeat => json!(()),
            AppEnvEventData::UserRequest(msg) => {
                json!(msg)
            }
            AppEnvEventData::Timer(data) => {
                json!(data)
            }
            AppEnvEventData::ToolResponse(results) => {
                json!(results)
            }
            AppEnvEventData::Error(info) => {
                json!({
                    "info":info.clone()
                })
            }
            AppEnvEventData::Interrupt(reason) => {
                json!({
                    "reason":reason.clone()
                })
            }
            AppEnvEventData::Interaction(data) => {
                json!(data)
            }
            AppEnvEventData::StackPop(title, content) => {
                json!({
                    "title": title,
                    "content": content
                })
            }
        }
    }

    pub fn from_json(type_name: EnvEventType, payload: serde_json::Value) -> AppEnvEventData {
        match type_name {
            EnvEventType::AgentResponse => {
                AppEnvEventData::AgentResponse(serde_json::from_value(payload).unwrap())
            }
            EnvEventType::Reflection => {
                AppEnvEventData::Reflection(serde_json::from_value(payload).unwrap())
            }
            EnvEventType::UserRequest => {
                let msg: UserMessage = serde_json::from_value(payload).unwrap();
                AppEnvEventData::UserRequest(msg)
            }
            EnvEventType::Heartbeat => AppEnvEventData::Heartbeat,
            EnvEventType::ToolResponse => {
                AppEnvEventData::ToolResponse(serde_json::from_value(payload).unwrap())
            }
            EnvEventType::Error => {
                let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
                let info = map.remove("info").unwrap_or_default();
                AppEnvEventData::Error(info)
            }
            EnvEventType::Timer => AppEnvEventData::Timer(serde_json::from_value(payload).unwrap()),
            EnvEventType::Interrupt => {
                let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
                let reason = map.remove("reason").unwrap_or_default();
                AppEnvEventData::Interrupt(reason)
            }
            EnvEventType::Interaction => {
                AppEnvEventData::Interaction(serde_json::from_value(payload).unwrap())
            }
            EnvEventType::StackPop => {
                let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
                let title = map.remove("title").unwrap_or_default();
                let content = map.remove("content").unwrap_or_default();
                AppEnvEventData::StackPop(title, content)
            }
        }
    }
    pub fn from_entity(event: entity::event::Model) -> AppEnvEvent {
        AppEnvEvent {
            id: event.id,
            data: from_json(event.event_type, event.data),
            force: event.force,
            time: event.time,
            size: event.size as _,
        }
    }
    pub fn to_type_enum(event: &AppEnvEventData) -> EnvEventType {
        match event {
            AppEnvEventData::AgentResponse(_) => EnvEventType::AgentResponse,
            AppEnvEventData::Heartbeat => EnvEventType::Heartbeat,
            AppEnvEventData::UserRequest(_) => EnvEventType::UserRequest,
            AppEnvEventData::Timer { .. } => EnvEventType::Timer,
            AppEnvEventData::ToolResponse(_) => EnvEventType::ToolResponse,
            AppEnvEventData::Error(_) => EnvEventType::Error,
            AppEnvEventData::Interrupt(_) => EnvEventType::Interrupt,
            AppEnvEventData::Interaction(_) => EnvEventType::Interaction,
            AppEnvEventData::Reflection(_) => EnvEventType::Reflection,
            AppEnvEventData::StackPop(_, _) => EnvEventType::StackPop,
        }
    }
}
