use sea_orm::entity::prelude::*;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "render_dependence")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(column_type = "Text", not_null, unique_key = "dep")]
    pub name: String,
    #[sea_orm(column_type = "Text", not_null, unique_key = "dep")]
    pub version: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub path: String,
    #[sea_orm(column_type = "Text", default_value = "js")]
    pub ext: String,
    pub editable: bool,
}

impl ActiveModelBehavior for ActiveModel {}
