use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::tool::core::ToolReturnValue;
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

#[derive(Debug, Clone)]
pub enum ToolCallEvent {
    //调用id组
    Start(Vec<CallItem>),
    Active(i64),
    Success(ToolCallResultItem),
    Failure(ToolCallResultItem),
    End,
}
