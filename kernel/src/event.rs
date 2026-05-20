use std::{collections::HashMap, fmt::Display, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    message::{Binary, Information},
    section::{Section, SectionTag},
    tool::core::ToolReturnValue,
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
    pub call_id: String,
    pub package: String,
    pub tool: String,
    pub values: Vec<ToolReturnValue>,
    pub status: ToolCallStatus,
    pub args: HashMap<String, serde_json::Value>,
    pub elapsed: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerData {
    pub id: i64,
    pub corn: String,
    pub payload: String,
    pub times: u32,
    pub trigger_times: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionData {
    pub section_id: i64,
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
    AgentResponse(Vec<Section>),
    Heartbeat,
    //用户消息
    UserRequest(UserMessage),
    //定时器
    Timer(TimerData),
    //工具调用结果
    ToolResponse(ToolCallDetailItem),
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
            EnvEvent::UserRequest(content) => {
                let mut list: Vec<Information> = content
                    .attachments
                    .into_iter()
                    .map(|p| Information::Image(p))
                    .collect();

                list.insert(0, content.content.into());
                list
            }
            EnvEvent::Heartbeat => {
                let one = "Heartbeat".into();
                vec![one]
            }
            EnvEvent::Timer(data) => {
                let times = if data.times == 0 {
                    "无限".to_string()
                } else {
                    data.times.to_string()
                };
                let head = format!(
                    "定时器id:{} 周期:{} 触发次数:{}/{}",
                    data.id, data.corn, data.trigger_times, times
                );
                let one = format!("{}\n附加信息:\n{}", head, data.payload).into();
                vec![one]
            }
            EnvEvent::ToolResponse(result) => {
                let mut list: Vec<Information> = vec![];
                let head = format!(
                    r#"<tool_reponse id="{}" status={} elapsed="{}">"#,
                    result.call_id, result.status, result.elapsed
                );
                list.push(Information::Text(head));
                for value in result.values {
                    match value {
                        ToolReturnValue::Ignore => {}

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
                 list.push(Information::Text("</tool_reponse>".to_string()));
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
                let one = format!("渲染ID:{}, 负载消息:{}", data.section_id, data.message).into();
                vec![one]
            }
        }
    }

    pub fn name(&self) -> String {
        let name = match self {
            EnvEvent::AgentResponse(_) => "agent响应",
            EnvEvent::Heartbeat => "agent心跳",
            EnvEvent::UserRequest(_) => "用户消息",
            EnvEvent::Timer { .. } => "定时器",
            EnvEvent::ToolResponse(_) => "工具调用结果",
            EnvEvent::Error(_) => "异常",
            EnvEvent::Interrupt(_) => "中断信号",
            EnvEvent::Interaction(_) => "UI交互事件",
        };
        name.to_string()
    }
    pub fn code(&self) -> String {
        let name = match self {
            EnvEvent::AgentResponse(_) => "agentResponse",
            EnvEvent::Heartbeat => "heartbeat",
            EnvEvent::UserRequest(_) => "userRequest",
            EnvEvent::Timer { .. } => "timer",
            EnvEvent::ToolResponse(_) => "toolResponse",
            EnvEvent::Error(_) => "error",
            EnvEvent::Interrupt(_) => "interrupt",
            EnvEvent::Interaction(_) => "interaction",
        };
        name.to_string()
    }
}

//======================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResultItem {
    pub id: i64,
    pub values: Vec<ToolReturnValue>,
    pub status: ToolCallStatus,
    pub elapsed: String,
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
    Start,
    Open(&'static SectionTag),
    Chunk(&'static SectionTag, String),
    Close(Section),
    End,
}
