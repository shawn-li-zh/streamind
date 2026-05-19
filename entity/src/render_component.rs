use sea_orm::entity::prelude::*;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "render_component")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(column_type = "Text", not_null, unique)]
    pub name: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub path: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub annotation: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub doc: String,
    pub author_type: String,
    pub author_name: String,
    pub preview_attrs: serde_json::Value,
    #[sea_orm(column_type = "Timestamp", not_null)]
    pub create_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
