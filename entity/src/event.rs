use std::fmt::Display;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::format::date_time;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "event")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub agent_id: u32,
    #[sea_orm(not_null)]
    pub step: i64,
    #[sea_orm(not_null)]
    pub state_id: i64,
    #[sea_orm(column_type = "Text", not_null)]
    pub event_type: EnvEventType,
    pub status: EnvEventStatus,
    #[sea_orm(not_null)]
    pub force: bool,
    #[sea_orm(column_type = "Text", not_null)]
    pub data: serde_json::Value,
    #[sea_orm(default_value = "0")]
    pub source_event_id: i64,
    #[sea_orm(column_type = "Timestamp", not_null)]
    #[serde(with = "date_time")]
    pub time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
pub enum EnvEventType {
    #[default]
    AgentReaction,
    AgentResponse,
    UserRequest,
    Heartbeat,
    ToolCallResult,
    Error,
    Timer,
    Interrupt,
    Interaction,
}
impl Display for EnvEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(&self).unwrap())
    }
}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
pub enum EnvEventStatus {
    #[default]
    Pending,
    Processing,
    Done,
}
