use crate::format::date_time;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "rule_item")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null, unique_key = "scope_rule")]
    pub scope_id: i64,
    #[sea_orm(column_type = "Text", not_null, unique_key = "scope_rule")]
    pub name: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub expr: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,

    //当前激活节点id
    #[sea_orm(default_value = "0")]
    pub state_id: i64,
    #[sea_orm(column_type = "Timestamp", not_null)]
    #[serde(with = "date_time")]
    pub create_time: chrono::DateTime<chrono::Local>,
    #[sea_orm(default_value = "0")]
    pub trigger_times: i32,
    #[serde(with = "date_time")]
    pub recently_time: chrono::DateTime<chrono::Local>,
    #[serde(with = "date_time")]
    pub expire_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
