use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{format::date_time, types::vector::Vector};
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, PartialEq, Serialize, Deserialize)]
#[sea_orm(table_name = "rule_concept")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null, unique_key = "scope_rule")]
    pub scope_id: i64,
    #[sea_orm(not_null, unique_key = "scope_rule")]
    pub name: String,

    #[sea_orm(not_null)]
    pub content: String,
    #[sea_orm(column_type = "Blob")] // 向量在底层可以作为二进制处理
    pub embedding: Option<Vector>,
    pub embedded: bool,
    #[sea_orm(column_type = "Text", not_null)]
    #[serde(with = "date_time")]
    pub create_time: chrono::DateTime<chrono::Local>,
}

impl ActiveModelBehavior for ActiveModel {}
