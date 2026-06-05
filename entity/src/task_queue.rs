use crate::format::date_time;
use chrono::Local;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Default, Serialize)]
#[sea_orm(table_name = "task_queue")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub task_type: String,
    #[sea_orm(not_null)]
    pub payload: serde_json::Value,
    #[sea_orm(not_null)]
    pub status: TaskStatus,
    pub error: Option<String>,
    pub elapsed: Option<String>,
    #[sea_orm(not_null)]
    #[serde(with = "date_time")]
    pub time: chrono::DateTime<Local>,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    #[default]
    Pending,
    Processing,
    Done,
}
