use sea_orm::{ColumnTrait, EntityTrait as _, QueryFilter, QuerySelect};

use crate::DB;

pub async fn get_by_id(id: i64) -> anyhow::Result<Option<entity::render_component::Model>> {
    let db = DB.get().unwrap();
    let model = entity::render_component::Entity::find_by_id(id)
        .one(db)
        .await?;
    Ok(model)
}

pub async fn range(
    start: u64,
    end: u64,
    key: Option<String>,
) -> anyhow::Result<Vec<entity::render_component::Model>> {
    let db = DB.get().unwrap();
    let mut query = entity::render_component::Entity::find();
    if let Some(key) = key {
        query = query.filter(entity::render_component::Column::Name.like(key));
    }
    let models = query.offset(start).limit(end).all(db).await?;
    Ok(models)
}
