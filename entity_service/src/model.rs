use entity::{
    model::{ModelCost, ModelStatus},
    types::array::StringArray,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde::Deserialize;

use crate::DB;

pub async fn query(
    name: Option<String>,
    modal: Option<String>,
    status: Option<ModelStatus>,
) -> anyhow::Result<Vec<entity::model::Model>> {
    let db = DB.get().unwrap();
    let mut find_builder = entity::model::Entity::find();

    if let Some(name) = name {
        if !name.is_empty() {
            find_builder =
                find_builder.filter(entity::model::Column::Name.like(format!("%{}%", name)));
        }
    }
    if let Some(modal) = modal {
        if !modal.is_empty() {
            find_builder = find_builder
                .filter(entity::model::Column::InputModals.like(format!("%{}%", modal)));
        }
    }
    if let Some(status) = status {
        find_builder = find_builder.filter(entity::model::Column::Status.eq(status));
    }

    let models = find_builder.all(db).await?;
    Ok(models)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Form {
    pub name: String,
    pub remark: Option<String>,
    pub api_url: String,
    pub api_key: Option<String>,
    pub input_modals: Vec<String>,
    pub output_modals: Vec<String>,
    pub cost: ModelCost,
    pub status: Option<ModelStatus>,
}
pub async fn update(db: &impl ConnectionTrait, id: u32, body: Form) -> anyhow::Result<()> {
    let model = entity::model::ActiveModel {
        id: Set(id),
        name: Set(body.name),
        remark: Set(body.remark.unwrap_or_default()),
        api_url: Set(body.api_url),
        api_key: Set(body.api_key.unwrap_or_default()),
        cost: Set(body.cost),
        input_modals: Set(StringArray(body.input_modals)),
        output_modals: Set(StringArray(body.output_modals)),
        status: Set(body.status.unwrap_or(ModelStatus::Active)),
        ..Default::default()
    }
    .update(db)
    .await?;
    Ok(())
}

pub async fn save(db: &impl ConnectionTrait, body: Form) -> anyhow::Result<entity::model::Model> {
    let model = entity::model::ActiveModel {
        name: Set(body.name),
        remark: Set(body.remark.unwrap_or_default()),
        api_url: Set(body.api_url),
        api_key: Set(body.api_key.unwrap_or_default()),
        cost: Set(body.cost),
        input_modals: Set(StringArray(body.input_modals)),
        output_modals: Set(StringArray(body.output_modals)),
        status: Set(body.status.unwrap_or(ModelStatus::Active)),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(model)
}
