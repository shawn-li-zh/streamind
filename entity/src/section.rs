use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "section")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub step: i64,
    #[sea_orm(column_type = "Text", not_null)]
    pub title: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub meta: String,
    #[sea_orm(not_null)]
    pub taken_time: u32,

}

impl ActiveModelBehavior for ActiveModel {}
