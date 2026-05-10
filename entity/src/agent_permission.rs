use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::format::date_time;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, PartialEq, Serialize, Deserialize)]
#[sea_orm(table_name = "agent_permission")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub agent_id: u32,
    pub code: String,
    pub sub_code: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub life: serde_json::Value,
    pub use_times: u32,
    pub is_valid: bool,
    pub purpose: String,
    #[sea_orm(column_type = "Timestamp", not_null)]
    #[serde(with = "date_time")]
    pub create_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}

