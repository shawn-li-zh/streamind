use sea_orm::entity::prelude::*;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Default, Serialize)]
#[sea_orm(table_name = "section")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub agent_id: u32,
    #[sea_orm(not_null)]
    pub event_id: i64,
    #[sea_orm(column_type = "Text", not_null)]
    pub name: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub payload: serde_json::Value,
    #[sea_orm(column_type = "Text", not_null)]
    pub value: serde_json::Value,
    #[sea_orm(not_null)]
    pub elapsed: String,
}

impl ActiveModelBehavior for ActiveModel {}
