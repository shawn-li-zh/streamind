use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "section")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u64,
    #[sea_orm(not_null)]
    pub bundle_id: u64,
    #[sea_orm(column_type = "Text", not_null)]
    pub title: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub meta: String,
    #[sea_orm(not_null)]
    pub taken_time: u32,

   #[sea_orm(belongs_to, from = "bundle_id", to = "id")]
    pub bundle: HasOne<super::event_bundle::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
