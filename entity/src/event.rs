use sea_orm::entity::prelude::*;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub agent_id: u32,
    #[sea_orm(not_null)]
    pub step: i64,
    #[sea_orm(not_null)]
    pub state_version: u32,
    #[sea_orm(column_type = "Text", not_null)]
    pub event_type: String,
    #[sea_orm(not_null)]
    pub force: bool,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,
    #[sea_orm(column_type = "Text", default_value = "")]
    pub response: String,
    #[sea_orm(default_value = "0")]
    pub source_event_id: i64,

    #[sea_orm(column_type = "Timestamp", not_null)]
    pub time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
