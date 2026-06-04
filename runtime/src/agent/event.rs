use kernel::{
    message::{Information, RoleMessage},
    mind::event::EnvEvent,
    mind::section::{Section, SectionTag},
    tool::event::ToolCallDetailItem,
};
use std::{collections::HashMap, fmt::Display, time::Duration};

use serde::{Deserialize, Serialize};

use kernel::{message::Binary, tool::core::ToolReturnValue};

use crate::agent::reflect::ReflectEvent;

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
pub enum SectionEvent {
    Start,
    Open(&'static SectionTag),
    Chunk(&'static SectionTag, String),
    Close(Section),
    End,
}

#[derive(Debug, Clone)]
pub enum AppEnvEventData {
    //agent响应
    AgentResponse(Vec<Section>),
    //心跳
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
    Reflection(ReflectEvent),
    StackPop(String, String),
}
impl AppEnvEventData {
    pub fn to_infos(self) -> Vec<Information> {
        match self {
            AppEnvEventData::AgentResponse(sections) => sections
                .into_iter()
                .filter(|s| match s.name.as_str() {
                    "push" | "reset" | "pop" => {
                        //栈操作标签，不作为消息内容进入上下文
                        return false;
                    }
                    _ => {
                        return true;
                    }
                })
                .map(|s| s.to_string().into())
                .collect(),
            AppEnvEventData::Reflection(event) => {
                let mut infos = vec![];
                infos.push(
                    format!(
                        "<reflect source_event_id={} level={}>",
                        event.source_event_id, event.level
                    )
                    .into(),
                );
                let datas: Vec<_> = event
                    .datas
                    .into_iter()
                    .map(|v| v.to_statement())
                    .map(|s| s.into())
                    .collect();
                infos.extend(datas);
                infos.push(format!("</reflect>",).into());
                infos
            }
            AppEnvEventData::UserRequest(content) => {
                let mut list: Vec<Information> = content
                    .attachments
                    .into_iter()
                    .map(|p| Information::Image(p))
                    .collect();

                list.insert(0, content.content.into());
                list
            }
            AppEnvEventData::Heartbeat => {
                let one = "Heartbeat".into();
                vec![one]
            }
            AppEnvEventData::Timer(data) => {
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
            AppEnvEventData::ToolResponse(result) => {
                let mut list: Vec<Information> = vec![];
                let head = format!(
                    r#"<tool_reponse id="{}" status="{}" elapsed="{}">"#,
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
            AppEnvEventData::Error(reason) => {
                let one = format!("{reason}").into();
                vec![one]
            }
            AppEnvEventData::Interrupt(reason) => {
                let one = format!("【中断信号:'{}'】", reason).into();
                vec![one]
            }
            AppEnvEventData::Interaction(data) => {
                let one = format!("渲染ID:{}, 负载消息:{}", data.section_id, data.message).into();
                vec![one]
            }
            AppEnvEventData::StackPop(title, content) => {
                let one = format!("子任务[{}]周期结束, 返回信息为:\n{}", title, content).into();
                vec![one]
            }
        }
    }

    pub fn name(&self) -> String {
        let name = match self {
            AppEnvEventData::AgentResponse(_) => "agent响应",
            AppEnvEventData::Heartbeat => "agent心跳",
            AppEnvEventData::UserRequest(_) => "用户消息",
            AppEnvEventData::Timer { .. } => "定时器",
            AppEnvEventData::ToolResponse(_) => "工具调用结果",
            AppEnvEventData::Error(_) => "异常",
            AppEnvEventData::Interrupt(_) => "中断信号",
            AppEnvEventData::Interaction(_) => "UI交互事件",
            AppEnvEventData::Reflection(_) => "条件反射",
            AppEnvEventData::StackPop(_, _) => "栈弹出",
        };
        name.to_string()
    }
    pub fn code(&self) -> String {
        let name = match self {
            AppEnvEventData::AgentResponse(_) => "agentResponse",
            AppEnvEventData::Heartbeat => "heartbeat",
            AppEnvEventData::UserRequest(_) => "userRequest",
            AppEnvEventData::Timer { .. } => "timer",
            AppEnvEventData::ToolResponse(_) => "toolResponse",
            AppEnvEventData::Error(_) => "error",
            AppEnvEventData::Interrupt(_) => "interrupt",
            AppEnvEventData::Interaction(_) => "interaction",
            AppEnvEventData::Reflection(_) => "reflection",
            AppEnvEventData::StackPop(_, _) => "stackPop",
        };
        name.to_string()
    }
    pub fn size(&self) -> usize {
        match self {
            AppEnvEventData::AgentResponse(sections) => sections
                .iter()
                .map(|s| s.content.len() + s.header.len())
                .sum::<usize>(),
            AppEnvEventData::UserRequest(content) => content.content.len(),
            AppEnvEventData::Heartbeat => 0,
            AppEnvEventData::Timer(data) => data.payload.len(),
            AppEnvEventData::ToolResponse(result) => result
                .values
                .iter()
                .map(|v| match v {
                    ToolReturnValue::Ignore => 0,
                    ToolReturnValue::Raw(r) => r.len(),
                    ToolReturnValue::Json(value) => value.to_string().len(),
                    ToolReturnValue::Image(binary) => binary.size,
                })
                .sum::<usize>(),
            AppEnvEventData::Interaction(interaction_data) => interaction_data.message.len(),
            AppEnvEventData::Error(e) => e.len(),
            AppEnvEventData::Interrupt(r) => r.len(),
            AppEnvEventData::Reflection(event) => event
                .datas
                .iter()
                .map(|s| s.content.len() + s.name.len())
                .sum::<usize>(),
            AppEnvEventData::StackPop(title, content) => title.len() + content.len(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct AppEnvEvent {
    pub id: i64,
    pub data: AppEnvEventData,
    pub force: bool,
    pub size: usize,
    pub time: chrono::DateTime<chrono::Local>,
}
impl EnvEvent for AppEnvEvent {
    fn size(&self) -> usize {
        self.size
    }
    fn to_message(&self) -> RoleMessage {
        if let AppEnvEventData::AgentResponse(sections) = &self.data {
            let infos: Vec<Information> = sections
                .into_iter()
                .map(|s| {
                    let full = s.to_string();
                    Information::Function(s.name.to_string(), full)
                })
                .collect();
            return RoleMessage::Assistant(infos, false);
        }
        let infos = self.to_infos();
        match &self.data {
            // AppEnvEventData::Error(_error) => RoleMessage::System(infos),
            AppEnvEventData::UserRequest(_user_message) => RoleMessage::User(infos),
            AppEnvEventData::ToolResponse(detail) => {
                RoleMessage::Tool(detail.call_id.clone(), infos)
            }
            _ => RoleMessage::User(infos),
        }
    }
}
impl AppEnvEvent {
    fn to_infos(&self) -> Vec<Information> {
        let time = self.time.format("%Y-%m-%d %H:%M:%S").to_string();
        let type_code = self.data.code();
        let head = format!(
            "<event id={},type=\"{}\",time=\"{}\">",
            self.id, type_code, time
        );
        let mut infos = self.data.clone().to_infos();
        infos.insert(0, head.into());
        infos.push("</event>".into());
        return infos;
    }
}
