use axum::response::sse;
use sea_orm::sea_query::prelude::rust_decimal::str;
use serde::Serialize;
use serde_json::json;
use streamind_core::tool::engine::ToolCallEvent;


use crate::agent::{
        event::{AgentEvent, AgentEventData},
        section::{Section, SectionEvent},
    };

pub enum WebEvent {
    Section(SectionEvent, i64),
    Agent(AgentEvent),
    ToolCall(ToolCallEvent),
}
impl WebEvent {
    pub fn to_sse_event(self) -> sse::Event {
        match self {
            WebEvent::Section(section_event, id) => match section_event {
                SectionEvent::Start => SsePayload::new("section", "start", ()).to_sse_event(),

                SectionEvent::Open(section_tag) => {
                    SsePayload::new("section", "open", section_tag.name).to_sse_event()
                }
                SectionEvent::Chunk(_, chunk) => {
                    SsePayload::new("section", "chunk", chunk).to_sse_event()
                }
                SectionEvent::Close(section) => {
                    let content = match section.tag.name {
                        "reply" | "think" => String::new(),
                        _ => section.content,
                    };
                    let data = json!({
                        "id":id,
                        "name":section.tag.name,
                        "content":content
                    });
                    SsePayload::new("section", "close", data).to_sse_event()
                }
                SectionEvent::End => SsePayload::new("section", "end", ()).to_sse_event(),
            },
            WebEvent::Agent(agent_event) => match agent_event.data {
                AgentEventData::AgentResponse
                | AgentEventData::UserInput(_)
                | AgentEventData::ToolCallResult(_) => unreachable!("调用出错"),

                AgentEventData::Timer(timer_data) => {
                    SsePayload::new("agent", "timer", timer_data).to_sse_event()
                }
                _ => todo!(),
            },
            WebEvent::ToolCall(tool_call_event) => match tool_call_event {
                ToolCallEvent::Start(ids) => {
                    SsePayload::new("tool_call", "start", json!({"ids":ids})).to_sse_event()
                }
                ToolCallEvent::Active(id, active) => {
                    SsePayload::new("tool_call", "active", json!({"id":id,"active":active}))
                        .to_sse_event()
                }
                ToolCallEvent::Success(id, result) => {
                    SsePayload::new("tool_call", "success", json!({"id":id,"result":result}))
                        .to_sse_event()
                }
                ToolCallEvent::Failure(id, result) => {
                    SsePayload::new("tool_call", "failure", json!({"id":id,"result":result}))
                        .to_sse_event()
                }
                ToolCallEvent::End => SsePayload::new("tool_call", "end", ()).to_sse_event(),
            },
        }
    }
}
#[derive(Debug, Serialize)]
pub struct SsePayload<T: Serialize> {
    pub r#type: &'static str,
    pub event: &'static str,
    pub data: T,
}
impl<T: Serialize> SsePayload<T> {
    pub fn new(r#type: &'static str, event: &'static str, data: T) -> Self {
        Self {
            r#type,
            event,
            data,
        }
    }
    pub fn to_sse_event(self) -> sse::Event {
        sse::Event::default().json_data(self).unwrap()
    }
}
