use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "rule_groups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(column_type = "Text", not_null, unique)]
    pub name: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub create_time: String,
}

impl ActiveModelBehavior for ActiveModel {}
