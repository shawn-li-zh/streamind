use sea_orm::entity::prelude::*;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, Default, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "render_state")]

pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub agent_id: u32,
    pub component_id: i64,
    pub response_event_id: i64,
    //在回复中是第几段
    pub section_index: u32,
    pub component_path: String,
    pub title: String,
    pub class: String,
    pub style: String,
    pub state: serde_json::Value,
}

impl ActiveModelBehavior for ActiveModel {}
