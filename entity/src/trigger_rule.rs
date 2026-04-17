use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel,Serialize)]
#[sea_orm(table_name = "trigger_rules")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub group_id: i64,
    #[sea_orm(column_type = "Text", not_null)]
    pub expr: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub hook: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub create_time: String,
    #[sea_orm(default_value = "0")]
    pub trigger_times: i32,
    #[sea_orm(column_type = "Text", not_null)]
    pub expire_time: String,
}

impl ActiveModelBehavior for ActiveModel {}
