use std::{collections::HashMap, fmt::Display, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    message::{Binary, Information},
    section::{Section, SectionTag},
    tool::core::{ToolReturn, ToolReturnValue},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolCallStatus {
    Undo,
    Success,
    Failure,
}
impl Display for ToolCallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sta = match self {
            ToolCallStatus::Undo => "Undo",
            ToolCallStatus::Success => "Success",
            ToolCallStatus::Failure => "Failure",
        };
        f.write_str(sta)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDetailItem {
    pub id: i64,
    pub package: String,
    pub tool: String,
    pub result: ToolReturn,
    pub status: ToolCallStatus,
    pub args: HashMap<String, serde_json::Value>,
    pub elapsed: String,
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
    pub attachments: Vec<Binary>,
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
    pub fn to_infos(self) -> Vec<Information> {
        match self {
            EnvEvent::AgentResponse(sections) => {
                sections.into_iter().map(|s| s.to_string().into()).collect()
            }
            EnvEvent::AgentReaction(sections) => {
                sections.into_iter().map(|s| s.to_string().into()).collect()
            }
            EnvEvent::UserRequest(content) => {
                let mut list: Vec<Information> = content
                    .attachments
                    .into_iter()
                    .map(|p| Information::Image(p))
                    .collect();

                list.insert(0, content.content.into());
                list
            }
            EnvEvent::Timer(data) => {
                let one = format!(
                    "附加信息：{},源事件id：{}",
                    data.attach, data.source_event_id
                )
                .into();
                vec![one]
            }
            EnvEvent::ToolCallResult(results) => {
                let list: Vec<Information> = results
                    .into_iter()
                    .map(|item| {
                        let mut list: Vec<Information> = vec![];
                        let head = format!("status={},elapsed={}", item.status, item.elapsed);
                        list.push(Information::Text(head));
                        for value in item.result.values {
                            match value {
                                ToolReturnValue::Raw(text) => {
                                    list.push(Information::Text(text));
                                }
                                ToolReturnValue::Json(value) => {
                                    list.push(Information::Text(value.to_string()));
                                }
                                ToolReturnValue::Image(bin) => {
                                    list.push(Information::Image(bin));
                                }
                            }
                        }
                        list
                    })
                    .flat_map(|items| items.into_iter())
                    .collect();
                list
            }
            EnvEvent::Error(reason) => {
                let one = format!("{reason}").into();
                vec![one]
            }
            EnvEvent::Interrupt(reason) => {
                let one = format!("【中断信号:'{}'】", reason).into();
                vec![one]
            }
            EnvEvent::Interaction(data) => {
                let one = format!(
                    "渲染状态ID:{}, 负载消息:{}",
                    data.render_state_id, data.message
                )
                .into();
                vec![one]
            }
        }
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
    pub result: ToolReturn,
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
