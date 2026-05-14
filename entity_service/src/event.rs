use chrono::Local;
use entity::event::EnvEventType;
use kernel::{
    agent::{AgentEvent, AgentInfo},
    event::{EnvEvent, ToolCallDetailItem},
    section::Section,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

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
            EnvEvent::AgentReaction(sections) => json!(sections),
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
            EnvEventType::AgentReaction => {
                EnvEvent::AgentReaction(serde_json::from_value(payload).unwrap())
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
            EnvEvent::AgentReaction(_) => EnvEventType::AgentReaction,
            EnvEvent::AgentResponse(_) => EnvEventType::AgentResponse,
            EnvEvent::Heartbeat=> EnvEventType::Heartbeat,
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
    agent_info: &AgentInfo,
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
        agent_id: Set(agent_info.id),
        state_id: Set(agent_info.state_id),
        step: Set(0),
        event_type: Set(event_type),
        force: Set(event.force),
        time: Set(event.time),
        data: Set(json),
        ..Default::default()
    };
    let model: entity::event::Model = event_model.insert(db).await?;
    event.id = model.id;
    Ok((event, model))
}

pub async fn update_tool_call_result(
    db: &impl ConnectionTrait,
    event_id: i64,
    details: Vec<ToolCallDetailItem>,
) -> anyhow::Result<()> {
    //更新section
    for ele in &details {
        crate::section::update_payload(db, ele.id, ele.clone()).await?;
    }
    //
    let data = serde_json::to_value(details)?;
    let model = entity::event::ActiveModel {
        data: Set(data),
        ..Default::default()
    };

    let result = entity::event::Entity::update_many()
        .set(model)
        .filter(entity::event::Column::Id.eq(event_id))
        .filter(entity::event::Column::EventType.eq(EnvEventType::ToolCallResult))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}

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
    let size = size.unwrap_or(30);
    let mut events = select
        .order_by_desc(entity::event::Column::Id)
        .limit(size)
        .all(db)
        .await?;
    events.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(events)
}
