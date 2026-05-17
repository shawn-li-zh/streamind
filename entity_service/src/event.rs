use chrono::Local;
use entity::{
    agent,
    event::{EnvEventStatus, EnvEventType},
    format::date_time,
};
use kernel::{
    agent::{AgentEvent, AgentInfo},
    event::{EnvEvent, ToolCallDetailItem},
    section::Section,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityOrSelect, EntityTrait,
    FromJsonQueryResult, FromQueryResult, QueryFilter, QueryOrder, QuerySelect,
};
use serde::Serialize;

use crate::DB;

pub mod convert {
    use std::collections::HashMap;

    use entity::event::EnvEventType;
    use kernel::{
        agent::AgentEvent,
        event::{EnvEvent, UserMessage},
    };
    use serde_json::json;

    pub fn from_entity(event: entity::event::Model) -> AgentEvent {
        AgentEvent {
            id: event.id,
            data: from_json(event.event_type, event.data),
            force: event.force,
            time: event.time,
        }
    }
    pub fn to_json(event: EnvEvent) -> serde_json::Value {
        match event {

            EnvEvent::AgentResponse(sections) => json!(sections),
            EnvEvent::Heartbeat => json!(()),
            EnvEvent::UserRequest(msg) => {
                json!(msg)
            }
            EnvEvent::Timer(data) => {
                json!(data)
            }
            EnvEvent::ToolCallResult(results) => {
                json!(results)
            }
            EnvEvent::Error(info) => {
                json!({
                    "info":info.clone()
                })
            }
            EnvEvent::Interrupt(reason) => {
                json!({
                    "reason":reason.clone()
                })
            }
            EnvEvent::Interaction(data) => {
                json!(data)
            }
        }
    }

    pub fn from_json(type_name: EnvEventType, payload: serde_json::Value) -> EnvEvent {
        match type_name {
            EnvEventType::AgentResponse => {
                EnvEvent::AgentResponse(serde_json::from_value(payload).unwrap())
            }

            EnvEventType::UserRequest => {
                let msg: UserMessage = serde_json::from_value(payload).unwrap();
                EnvEvent::UserRequest(msg)
            }
            EnvEventType::Heartbeat => EnvEvent::Heartbeat,
            EnvEventType::ToolCallResult => {
                EnvEvent::ToolCallResult(serde_json::from_value(payload).unwrap())
            }
            EnvEventType::Error => {
                let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
                let info = map.remove("info").unwrap_or_default();
                EnvEvent::Error(info)
            }
            EnvEventType::Timer => EnvEvent::Timer(serde_json::from_value(payload).unwrap()),
            EnvEventType::Interrupt => {
                let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
                let reason = map.remove("reason").unwrap_or_default();
                EnvEvent::Interrupt(reason)
            }
            EnvEventType::Interaction => {
                EnvEvent::Interaction(serde_json::from_value(payload).unwrap())
            }
        }
    }

    pub fn to_type_enum(event: &EnvEvent) -> EnvEventType {
        match event {

            EnvEvent::AgentResponse(_) => EnvEventType::AgentResponse,
            EnvEvent::Heartbeat => EnvEventType::Heartbeat,
            EnvEvent::UserRequest(_) => EnvEventType::UserRequest,
            EnvEvent::Timer { .. } => EnvEventType::Timer,
            EnvEvent::ToolCallResult(_) => EnvEventType::ToolCallResult,
            EnvEvent::Error(_) => EnvEventType::Error,
            EnvEvent::Interrupt(_) => EnvEventType::Interrupt,
            EnvEvent::Interaction(_) => EnvEventType::Interaction,
        }
    }
}

pub async fn save(
    db: &impl ConnectionTrait,
    agent_id: u32,
    state_id: i64,
    event_data: EnvEvent,
    force: bool,
) -> anyhow::Result<(AgentEvent, entity::event::Model)> {
    //保存数据库
    let mut event = AgentEvent {
        id: 0,
        data: event_data.clone(),
        force,
        time: Local::now(),
    };
    let event_type = convert::to_type_enum(&event_data);
    let json = convert::to_json(event_data);
    let event_model = entity::event::ActiveModel {
        agent_id: Set(agent_id),
        state_id: Set(state_id),
        step: Set(0),
        event_type: Set(event_type),
        force: Set(event.force),
        time: Set(event.time),
        data: Set(json),
        status: Set(EnvEventStatus::Pending),
        ..Default::default()
    };
    let model: entity::event::Model = event_model.insert(db).await?;
    event.id = model.id;
    Ok((event, model))
}

// pub async fn update_tool_call_result(
//     db: &impl ConnectionTrait,
//     event_id: i64,
//     details: Vec<ToolCallDetailItem>,
// ) -> anyhow::Result<()> {
//     //更新section
//     for ele in &details {
//         crate::section::update_value(db, ele.id, ele.clone()).await?;
//     }
//     //
//     let data = serde_json::to_value(details)?;
//     let model = entity::event::ActiveModel {
//         data: Set(data),
//         status: Set(EnvEventStatus::Pending),
//         ..Default::default()
//     };

//     let result = entity::event::Entity::update_many()
//         .set(model)
//         .filter(entity::event::Column::Id.eq(event_id))
//         .filter(entity::event::Column::EventType.eq(EnvEventType::ToolCallResult))
//         .exec(db)
//         .await?;
//     if result.rows_affected != 1 {
//         return Err(anyhow::anyhow!("更新失败"));
//     }
//     Ok(())
// }

pub async fn update_agent_response(
    db: &impl ConnectionTrait,
    event_id: i64,
    event_type: EnvEventType,
    sections: Vec<Section>,
) -> anyhow::Result<()> {
    if sections.is_empty() {
        //删除
        let result = entity::event::Entity::delete_many()
            .filter(entity::event::Column::Id.eq(event_id))
            .filter(entity::event::Column::EventType.eq(event_type.clone()))
            .exec(db)
            .await?;
        return if result.rows_affected == 1 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("删除失败:{},{}", event_type, event_id))
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
        .filter(entity::event::Column::EventType.eq(event_type))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}
pub async fn update_event_status(
    db: &impl ConnectionTrait,
    event_ids: Vec<i64>,
    status: EnvEventStatus,
) -> anyhow::Result<()> {
    let model = entity::event::ActiveModel {
        status: Set(status),
        ..Default::default()
    };
    entity::event::Entity::update_many()
        .filter(entity::event::Column::Id.is_in(event_ids))
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
