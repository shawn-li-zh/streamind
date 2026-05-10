use sea_orm::EntityTrait as _;

use crate::DB;

pub async fn get_by_id(id: i64) -> anyhow::Result<Option<entity::render_component::Model>> {
    let db = DB.get().unwrap();
    let model = entity::render_component::Entity::find_by_id(id)
        .one(db)
        .await?;
    Ok(model)
}
