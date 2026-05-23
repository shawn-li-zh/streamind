pub mod convert {
    use std::collections::HashMap;

    use entity::event::EnvEventType;
    use runtime::agent::event::{AppEnvEvent, AppEnvEventData, UserMessage};
    use serde_json::json;

    use super::*;

    pub fn to_json(event: &AppEnvEventData) -> serde_json::Value {
        match event {
            AppEnvEventData::AgentResponse(sections) => json!(sections),
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
        }
    }

    pub fn from_json(type_name: EnvEventType, payload: serde_json::Value) -> AppEnvEventData {
        match type_name {
            EnvEventType::AgentResponse => {
                AppEnvEventData::AgentResponse(serde_json::from_value(payload).unwrap())
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
        }
    }
    pub fn from_entity(event: entity::event::Model) -> AppEnvEvent {
        AppEnvEvent {
            id: event.id,
            data: from_json(event.event_type, event.data),
            force: event.force,
            time: event.time,
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
        }
    }
}
