use axum::response::sse;

use kernel::tool::event::ToolCallEvent;
use runtime::agent::event::{AppEnvEventData, SectionEvent};
use serde::Serialize;
use serde_json::json;

pub enum WebEvent {
    Env(AppEnvEventData, i64),
    Section(SectionEvent, i64),
    ToolCall(ToolCallEvent, i64),
    Connected,
}
impl WebEvent {
    pub fn to_sse_event(self) -> sse::Event {
        match self {
            WebEvent::Section(section_event, bind_event_id) => match section_event {
                SectionEvent::Start => {
                    SsePayload::new("section", "start", json!(()), bind_event_id).to_sse_event()
                }

                SectionEvent::Open(section_tag) => {
                    SsePayload::new("section", "open", section_tag.name, bind_event_id)
                        .to_sse_event()
                }
                SectionEvent::Chunk(tag, chunk) => SsePayload::new(
                    "section",
                    "chunk",
                    json!({
                        "tag": tag.name,
                        "chunk": chunk
                    }),
                    bind_event_id,
                )
                .to_sse_event(),
                SectionEvent::Close(section) => {
                    // let content = match section.tag_name.as_str() {
                    //     "reply" | "think" => String::new(),
                    //     _ => section.content,
                    // };
                    let data = json!(section);
                    SsePayload::new("section", "close", data, bind_event_id).to_sse_event()
                }
                SectionEvent::End => {
                    SsePayload::new("section", "end", json!(()), bind_event_id).to_sse_event()
                }
            },
            WebEvent::Env(event, bind_event_id) => match event {
                AppEnvEventData::AgentResponse(_)
                | AppEnvEventData::Interaction(_)
                | AppEnvEventData::Heartbeat
                | AppEnvEventData::ToolResponse(_) => {
                    unreachable!("调用出错")
                }
                AppEnvEventData::UserRequest(user) => {
                    SsePayload::new("env", "userRequest", user, bind_event_id).to_sse_event()
                }
                AppEnvEventData::Timer(timer_data) => {
                    SsePayload::new("env", "timer", timer_data, bind_event_id).to_sse_event()
                }
                AppEnvEventData::Error(err) => {
                    SsePayload::new("env", "error", err, bind_event_id).to_sse_event()
                }
                AppEnvEventData::Interrupt(reason) => {
                    SsePayload::new("env", "interrupt", reason, bind_event_id).to_sse_event()
                }
            },
            WebEvent::ToolCall(tool_call_event, bind_event_id) => match tool_call_event {
                ToolCallEvent::Start(items) => {
                    SsePayload::new("tool_call", "start", json!(items), bind_event_id)
                        .to_sse_event()
                }
                ToolCallEvent::Active(info) => {
                    SsePayload::new("tool_call", "active", json!(info), bind_event_id)
                        .to_sse_event()
                }
                ToolCallEvent::Success(item) => {
                    SsePayload::new("tool_call", "success", json!(item), bind_event_id)
                        .to_sse_event()
                }
                ToolCallEvent::Failure(item) => {
                    SsePayload::new("tool_call", "failure", json!(item), bind_event_id)
                        .to_sse_event()
                }
                ToolCallEvent::End => {
                    SsePayload::new("tool_call", "end", (), bind_event_id).to_sse_event()
                }
            },
            WebEvent::Connected => SsePayload::new("server", "connected", (), 0).to_sse_event(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SsePayload<T: Serialize> {
    pub bind_event_id: i64,
    pub r#type: &'static str,
    pub event: &'static str,
    pub data: T,
}
impl<T: Serialize> SsePayload<T> {
    pub fn new(event: &'static str, r#type: &'static str, data: T, bind_event_id: i64) -> Self {
        Self {
            r#type,
            event,
            data,
            bind_event_id,
        }
    }
    pub fn to_sse_event(self) -> sse::Event {
        sse::Event::default().json_data(self).unwrap()
    }
}
