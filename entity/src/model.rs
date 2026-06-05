use std::default;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize, de};

use crate::types::array::StringArray;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "model")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u32,
    #[sea_orm(not_null)]
    pub name: String,
    pub remark: String,
    #[sea_orm(not_null)]
    pub api_url: String,
    #[sea_orm(not_null)]
    pub api_key: String,
    pub input_modals: StringArray,
    pub output_modals: StringArray,
    #[sea_orm(not_null, default_value = 0)]
    pub consumed_tokens: i64,
    pub status: ModelStatus,
    pub cost: ModelCost,
    #[sea_orm(not_null, default_value = "none")]
    pub reasoning_effort: ReasoningEffort,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "lowercase"
)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    #[default]
    Active,
    Inactive,
}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "lowercase"
)]
#[serde(rename_all = "lowercase")]
pub enum ModelCost {
    #[default]
    Low,
    Medium,
    High,
}




#[derive(EnumIter, DeriveActiveEnum, Clone, Serialize, Debug, Deserialize, PartialEq, Default)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "lowercase"
)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
}
