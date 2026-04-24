use crate::format::date_time;
use chrono::Local;
use sea_orm::entity::prelude::*;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Default, Serialize)]
#[sea_orm(table_name = "state")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub agent_id: u32,
    pub pos: u32,
    // pub step: i64,
    #[sea_orm(not_null, default_value = false)]
    pub dumped: bool,
    #[sea_orm(column_type = "Text", not_null)]
    pub title: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,

    #[sea_orm(not_null)]
    #[serde(with = "date_time")]
    pub time: chrono::DateTime<Local>,
}

impl ActiveModelBehavior for ActiveModel {}
