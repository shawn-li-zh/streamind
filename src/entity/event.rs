use sea_orm::entity::prelude::*;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u64,
    #[sea_orm(not_null)]
    pub agent_id: u32,
    #[sea_orm(not_null)]
    pub bundle_id: u32,
    #[sea_orm(column_type = "Text", not_null)]
    pub event_type: String,
    #[sea_orm(not_null)]
    pub force: bool,
    #[sea_orm(column_type = "Text", not_null)]
    pub content: String,
    #[sea_orm(column_type = "Text", default_value = "")]
    pub response: String,
    pub source_event_id: Option<u64>,

    #[sea_orm(column_type = "Timestamp", not_null)]
    pub time: chrono::DateTime<chrono::Local>,
    #[sea_orm(belongs_to, from = "bundle_id", to = "id")]
    pub bundle: HasOne<super::event_bundle::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
