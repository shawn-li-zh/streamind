use sea_orm::entity::prelude::*;

use crate::types::array::StringArray;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "agent")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    #[sea_orm(not_null)]
    pub state_version: u32,
    #[sea_orm(column_type = "Text", not_null)]
    pub model: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub name: String,
    #[sea_orm(not_null, default_value = 0)]
    pub step: i64,

    #[sea_orm(column_type = "Text", default_value = "[]")]
    pub tool_packages: StringArray,
    #[sea_orm(column_type = "Timestamp", not_null)]
    pub create_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
