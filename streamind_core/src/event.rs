use std::{collections::HashMap, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use crate::section::{Section, SectionTag};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolCallStatus {
    Undo,
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDetailItem {
    pub id: i64,
    pub package: String,
    pub tool: String,
    pub content: String,
    pub status: ToolCallStatus,
    pub args: HashMap<String, serde_json::Value>,
    pub time: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerData {
    pub attach: String,
    pub source_event_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionData {
    pub render_state_id: i64,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: String,
    pub attachments: Vec<PathBuf>,
}

pub enum Reaction {
    Think { content: String, time: Duration },
    ToolCall { content: String, time: Duration },
}

#[derive(Debug, Clone)]
pub enum EnvEvent {
    //agent响应
    AgentReaction(Vec<Section>),
    AgentResponse(Vec<Section>),
    //用户消息
    UserRequest(UserMessage),
    //定时器
    Timer(TimerData),
    //工具调用结果
    ToolCallResult(Vec<ToolCallDetailItem>),
    //UI交互事件
    Interaction(InteractionData),
    //异常
    Error(String),
    //中断信号
    Interrupt(String),
}
impl EnvEvent {
    pub fn to_message(self) -> (String, Vec<PathBuf>) {
        let content = match self {
            EnvEvent::AgentResponse(sections) => {
                let s: Vec<String> = sections.into_iter().map(|s| s.to_string()).collect();
                s.join("\n")
            }
            EnvEvent::AgentReaction(sections) => {
                let s: Vec<String> = sections.into_iter().map(|s| s.to_string()).collect();
                s.join("\n")
            }
            EnvEvent::UserRequest(content) => {
                return (content.content, content.attachments);
            }
            EnvEvent::Timer(data) => {
                format!(
                    "附加信息：{},源事件id：{}",
                    data.attach, data.source_event_id
                )
            }
            EnvEvent::ToolCallResult(results) => {
                let mut content = String::new();
                for item in results {
                    content.push_str(&item.content);
                    content.push_str("\n---\n");
                }
                content
            }
            EnvEvent::Error(reason) => format!("{}", reason),
            EnvEvent::Interrupt(reason) => format!("【中断信号:'{}'】", reason),
            EnvEvent::Interaction(data) => {
                format!(
                    "渲染状态ID:{}, 负载消息:{}",
                    data.render_state_id, data.message
                )
            }
        };
        (content, vec![])
    }

    pub fn name(&self) -> String {
        let name = match self {
            EnvEvent::AgentReaction(_) => "agent反应",
            EnvEvent::AgentResponse(_) => "agent响应",
            EnvEvent::UserRequest(_) => "用户消息",
            EnvEvent::Timer { .. } => "定时器",
            EnvEvent::ToolCallResult(_) => "工具调用结果",
            EnvEvent::Error(_) => "异常",
            EnvEvent::Interrupt(_) => "中断信号",
            EnvEvent::Interaction(_) => "UI交互事件",
        };
        name.to_string()
    }
}

//======================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResultItem {
    pub id: i64,
    pub content: String,
    pub status: ToolCallStatus,
    pub time: Duration,
}
#[derive(Debug, Clone, Serialize)]
pub struct CallItem {
    pub id: i64,
    pub package: &'static str,
    pub tool: &'static str,
    pub args: HashMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ActiveInfo {
    pub id: i64,
    pub tip: String,
}
#[derive(Debug, Clone)]
pub enum ToolCallEvent {
    //调用id组
    Start(Vec<CallItem>),
    // section id+工具活动描述
    Active(i64),
    // section id+调用结果
    Success(ToolCallResultItem),
    Failure(ToolCallResultItem),
    End,
}
#[derive(Debug, Clone)]
pub enum SectionEvent {
    Start(i64, i64),
    Open(&'static SectionTag),
    Chunk(&'static SectionTag, String),
    Close(Section),
    End(i64, i64),
}
