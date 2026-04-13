use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "agent")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(column_type = "Text", not_null)]
    pub model: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub name: String,
    #[sea_orm(column_type = "Text", default_value = "")]
    pub memory: String,
    #[sea_orm(column_type = "Text", default_value = "")]
    pub goal: String,
    #[sea_orm(column_type = "Timestamp", not_null)]
    pub create_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
