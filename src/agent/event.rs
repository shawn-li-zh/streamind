use std::{collections::HashMap, path::PathBuf};

use chrono::Local;
use entity::{event::EnvEventType, section};

use serde_json::json;
use streamind_core::event::{EnvEvent, UserMessage};

//事件
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub id: i64,
    pub agent_id: u32,
    pub data: EnvEvent,
    //是否强制执行
    pub force: bool,
    pub time: chrono::DateTime<Local>,
}

impl AgentEvent {
    pub fn to_message(&self) -> String {
        let time = self.time.format("%Y-%m-%d %H:%M:%S").to_string();
        let head = format!("id={},type={},time={}", self.id, self.data.name(), time);
        let (content, paths) = self.data.clone().to_message();
        return format!("{}\nContent:\n{}", head, content);
    }
    pub fn from_entity(event: entity::event::Model) -> Self {
        AgentEvent {
            id: event.id,
            agent_id: event.agent_id,
            data: from_json(event.event_type, event.data),
            force: event.force,
            time: event.time,
        }
    }
}

pub fn to_json(event: EnvEvent) -> serde_json::Value {
    match event {
        EnvEvent::AgentResponse(sections) => json!(sections),
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
    }
}

pub fn to_type_enum(event: &EnvEvent) -> EnvEventType {
    match event {
        EnvEvent::AgentResponse(_) => EnvEventType::AgentResponse,
        EnvEvent::UserRequest(_) => EnvEventType::UserRequest,
        EnvEvent::Timer { .. } => EnvEventType::Timer,
        EnvEvent::ToolCallResult(_) => EnvEventType::ToolCallResult,
        EnvEvent::Error(_) => EnvEventType::Error,
        EnvEvent::Interrupt(_) => EnvEventType::Interrupt,
    }
}
