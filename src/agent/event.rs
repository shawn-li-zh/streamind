use std::collections::HashMap;

use chrono::Local;
use entity::event::EventTypeName;
use sea_orm::sea_query::prelude::rust_decimal::str;
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResultItem {
    pub id: i64,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerData {
    pub attach: String,
    pub source_event_id: u64,
}
#[derive(Debug, Clone)]
pub enum AgentEventData {
    //agent响应 占位
    AgentResponse,
    //用户输入
    UserInput(String),

    //定时器
    Timer(TimerData),
    //工具调用结果
    ToolCallResult(Vec<ToolCallResultItem>),
    //异常
    Error(String),
    //中断信号
    Interrupt(String),
}
impl AgentEventData {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEventData::AgentResponse => "agent响应",
            AgentEventData::UserInput(_) => "用户输入",
            AgentEventData::Timer { .. } => "定时器",
            AgentEventData::ToolCallResult(_) => "工具调用结果",
            AgentEventData::Error(_) => "异常",
            AgentEventData::Interrupt(_) => "中断信号",
        }
    }
    pub fn type_enum(&self) -> EventTypeName {
        match self {
            AgentEventData::AgentResponse => EventTypeName::AgentResponse,
            AgentEventData::UserInput(_) => EventTypeName::UserInput,
            AgentEventData::Timer { .. } => EventTypeName::Timer,
            AgentEventData::ToolCallResult(_) => EventTypeName::ToolCallResult,
            AgentEventData::Error(_) => EventTypeName::Error,
            AgentEventData::Interrupt(_) => todo!(),
        }
    }
    pub fn content(&self) -> String {
        match self {
            AgentEventData::AgentResponse => String::new(),
            AgentEventData::UserInput(content) => content.clone(),
            AgentEventData::Timer(data) => {
                format!(
                    "附加信息：{},源事件id：{}",
                    data.attach, data.source_event_id
                )
            }
            AgentEventData::ToolCallResult(results) => {
                let mut content = String::new();
                for item in results {
                    content.push_str(&item.content);
                    content.push_str("\n---\n");
                }
                content
            }
            AgentEventData::Error(reason) => format!("{}", reason),
            AgentEventData::Interrupt(reason) => format!("【中断信号:'{}'】", reason),
        }
    }
    pub fn json(&self) -> serde_json::Value {
        match self {
            AgentEventData::AgentResponse => serde_json::Value::Null,
            AgentEventData::UserInput(content) => {
                json!({
                    "content":content.clone()
                })
            }
            AgentEventData::Timer(data) => {
                json!(data)
            }
            AgentEventData::ToolCallResult(results) => {
                json!(results)
            }
            AgentEventData::Error(info) => {
                json!({
                    "info":info.clone()
                })
            }
            AgentEventData::Interrupt(reason) => {
                json!({
                    "reason":reason.clone()
                })
            }
        }
    }
}
//事件
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub id: i64,
    pub agent_id: u32,
    pub data: AgentEventData,
    //是否强制执行
    pub force: bool,
    pub time: chrono::DateTime<Local>,
}

impl AgentEvent {
    pub fn to_message(&self) -> String {
        let time = self.time.format("%Y-%m-%d %H:%M:%S").to_string();
        let head = format!("id={},type={},time={}", self.id, self.data.name(), time);
        let content = self.data.content();
        return format!("{}\nContent:\n{}", head, content);
    }
    pub fn from_entity(event: entity::event::Model) -> Self {
        AgentEvent {
            id: event.id,
            agent_id: event.agent_id,
            data: to_event_data(event.event_type, event.payload),
            force: event.force,
            time: event.time,
        }
    }
}

fn to_event_data(type_name: EventTypeName, payload: serde_json::Value) -> AgentEventData {
    match type_name {
        EventTypeName::AgentResponse => AgentEventData::AgentResponse,
        EventTypeName::UserInput => {
            let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
            let content = map.remove("content").unwrap_or_default();
            AgentEventData::UserInput(content)
        }

        EventTypeName::ToolCallResult => {
            AgentEventData::ToolCallResult(serde_json::from_value(payload).unwrap())
        }
        EventTypeName::Error => {
            let mut map: HashMap<String, String> = serde_json::from_value(payload).unwrap();
            let info = map.remove("info").unwrap_or_default();
            AgentEventData::Error(info)
        }
        EventTypeName::Timer => AgentEventData::Timer(serde_json::from_value(payload).unwrap()),
    }
}
