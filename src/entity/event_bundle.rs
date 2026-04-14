use sea_orm::entity::prelude::*;
#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "event_bundle")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(not_null)]
    pub agent_id: i64,
    #[sea_orm(not_null)]
    pub first_event_id: i64,
    #[sea_orm(not_null)]
    pub least_event_id: i64,

    #[sea_orm(has_many)]
    pub sections: HasMany<super::section::Entity>,
    #[sea_orm(has_many)]
    pub events: HasMany<super::event::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
