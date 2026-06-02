use entity::agent::Entity;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait as _, ExprTrait, PaginatorTrait,
    QueryFilter, QuerySelect,
};
use serde::{Deserialize, Serialize};

use crate::DB;

pub async fn get_by_id(id: i64) -> anyhow::Result<Option<entity::render_component::Model>> {
    let db = DB.get().unwrap();
    let model = entity::render_component::Entity::find_by_id(id)
        .one(db)
        .await?;
    Ok(model)
}
pub async fn get_by_name(name: &str) -> anyhow::Result<Option<entity::render_component::Model>> {
    let db = DB.get().unwrap();
    let model = entity::render_component::Entity::find()
        .filter(entity::render_component::Column::Name.eq(name))
        .one(db)
        .await?;
    Ok(model)
}

pub async fn total(key: Option<String>) -> anyhow::Result<u64> {
    let db = DB.get().unwrap();
    let mut query = entity::render_component::Entity::find();
    if let Some(key) = key {
        query = query.filter(
            entity::render_component::Column::Name.like(&key).or(
                entity::render_component::Column::Doc
                    .like(&key)
                    .or(entity::render_component::Column::Annotation.like(&key)),
            ),
        );
    }
    let count = query.count(db).await?;
    Ok(count)
}

pub async fn page(
    page: u64,
    size: u64,
    key: Option<String>,
) -> anyhow::Result<Vec<entity::render_component::Model>> {
    let db = DB.get().unwrap();
    let mut query = entity::render_component::Entity::find();
    if let Some(key) = key.filter(|v| !v.trim().is_empty()) {
        query = query.filter(
            entity::render_component::Column::Name.like(&key).or(
                entity::render_component::Column::Doc
                    .like(&key)
                    .or(entity::render_component::Column::Annotation.like(&key)),
            ),
        );
    }
    let start = (page - 1) * size;
    let models = query.offset(start).limit(size).all(db).await?;
    Ok(models)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Body {
    pub name: String,
    pub path: String,
    pub annotation: String,
    pub doc: String,
    pub preview_attrs: serde_json::Value,
}
pub async fn create(req: Body) -> anyhow::Result<i64> {
    let db = DB.get().unwrap();
    let model = entity::render_component::ActiveModel {
        name: Set(req.name),
        path: Set(req.path),
        annotation: Set(req.annotation),
        doc: Set(req.doc),
        author_type: Set("system".to_string()),
        author_name: Set("system".to_string()),
        preview_attrs: Set(req.preview_attrs),
        create_time: Set(chrono::Local::now()),
        ..Default::default()
    };
    let model = model.insert(db).await?;
    Ok(model.id)
}

pub async fn update(id: i64, req: Body) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let res = entity::render_component::Entity::update_many()
        .set(entity::render_component::ActiveModel {
            name: Set(req.name),
            path: Set(req.path),
            annotation: Set(req.annotation),
            doc: Set(req.doc),
            preview_attrs: Set(req.preview_attrs),
            ..Default::default()
        })
        .filter(entity::render_component::Column::Id.eq(id))
        .exec(db)
        .await?;

    if res.rows_affected != 1 {
        anyhow::bail!("更新失败")
    }
    Ok(())
}
