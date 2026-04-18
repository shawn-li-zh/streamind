use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "item")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    #[sea_orm(not_null, unique_key = "uk")]
    pub agent_id: u32,
    #[sea_orm(not_null, unique_key = "uk")]
    pub group: String,
    #[sea_orm(not_null, unique_key = "uk")]
    pub name: String,
    #[sea_orm(not_null)]
    pub content: String,
    #[sea_orm(not_null, default_value = true)]
    pub editable: bool,
}

impl ActiveModelBehavior for ActiveModel {}
