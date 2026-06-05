use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::format::{date_time, date_time_option};
use crate::types::array::StringArray;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, PartialEq, Serialize, Deserialize)]
#[sea_orm(table_name = "agent")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    pub model_id: u32,
    #[sea_orm(default_value=0)]
    pub memory_model_id: u32,
    #[sea_orm(not_null, default_value = 0)]
    pub parent_id: u32,
    #[sea_orm(not_null, default_value = 0)]
    pub state_id: i64,
    #[sea_orm(column_type = "Text", not_null)]
    pub name: String,
    #[sea_orm(column_type = "Text", not_null, unique)]
    pub code: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub persona: String,
    pub workspace: String,
    pub avatar: String,
    #[sea_orm(column_type = "Text", default_value = "[]")]
    pub tool_packages: StringArray,
    #[sea_orm(column_type = "Text", default_value = "[]")]
    pub rule_groups: StringArray,
    pub status: AgentStatus,
    #[sea_orm(column_type = "Timestamp")]
    #[serde(with = "date_time_option")]
    pub startup_time: Option<chrono::DateTime<chrono::Local>>,

    pub sleep_duration: Option<String>,
    #[sea_orm(column_type = "Timestamp", not_null)]
    #[serde(with = "date_time")]
    pub create_time: chrono::DateTime<chrono::Local>,
    #[sea_orm(ignore)]
    pub children: Vec<Model>,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    #[default]
    Idle, //空闲
    Sleep,      //睡眠
    Processing, //处理任务中
    Waiting,
    Paused, //暂停 需要用户手动恢复到空闲
}
