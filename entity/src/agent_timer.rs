use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::format::date_time;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, PartialEq, Serialize, Deserialize)]
#[sea_orm(table_name = "agent_timer")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub agent_id: u32,
    pub corn: String,
    #[sea_orm(default = "0")]
    pub times: u32,
    #[sea_orm(default = "0")]
    pub trigger_times: u32,
    pub payload: String,
    pub disused: bool,
    #[sea_orm(column_type = "Timestamp", not_null)]
    #[serde(with = "date_time")]
    pub create_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
