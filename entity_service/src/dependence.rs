use std::{collections::HashMap, io::Write, path::PathBuf};

use futures::StreamExt;
use reqwest::Url;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait as _, QueryFilter};
use serde_json::json;

use crate::DB;
pub async fn create(
    name: String,
    version: String,
    path: String,
) -> anyhow::Result<entity::render_dependence::Model> {
    let db = DB.get().unwrap();
    let model = entity::render_dependence::ActiveModel {
        name: Set(name),
        version: Set(version),
        path: Set(path),
        editable: Set(true),
        ..Default::default()
    };
    let model = model.insert(db).await?;
    rewrite_manifest().await?;
    Ok(model)
}

pub async fn update(id: i64, name: String, version: String, path: String) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let res = entity::render_dependence::Entity::update_many()
        .set(entity::render_dependence::ActiveModel {
            name: Set(name),
            version: Set(version),
            path: Set(path),
            ..Default::default()
        })
        .filter(entity::render_dependence::Column::Id.eq(id))
        .filter(entity::render_dependence::Column::Editable.eq(true))
        .exec(db)
        .await?;
    if res.rows_affected != 1 {
        anyhow::bail!("Dependence not found");
    }
    rewrite_manifest().await?;
    Ok(())
}
pub async fn list(key: Option<String>) -> anyhow::Result<Vec<entity::render_dependence::Model>> {
    let db = DB.get().unwrap();
    let mut query = entity::render_dependence::Entity::find();
    if let Some(key) = key {
        query = query.filter(entity::render_dependence::Column::Name.like(key));
    }
    let models = query.all(db).await?;
    Ok(models)
}

pub async fn get(id: i32) -> anyhow::Result<entity::render_dependence::Model> {
    let db = DB.get().unwrap();
    let model = entity::render_dependence::Entity::find_by_id(id)
        .one(db)
        .await?;
    if model.is_none() {
        anyhow::bail!("Dependence not found");
    }
    Ok(model.unwrap())
}

pub async fn remove(id: i32) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    entity::render_dependence::Entity::delete_by_id(id)
        .filter(entity::render_dependence::Column::Editable.eq(true))
        .exec(db)
        .await?;
    rewrite_manifest().await?;
    Ok(())
}
pub async fn rewrite_manifest() -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let models = entity::render_dependence::Entity::find().all(db).await?;
    if models.is_empty() {
        return Ok(());
    }
    //按名称分组，按版本排序
    let groups = models.into_iter().fold(HashMap::new(), |mut acc, item| {
        let models = acc.entry(item.name.clone()).or_insert(vec![]);
        models.push(item);
        acc
    });
    let mut map = HashMap::new();
    for (name, mut models) in groups {
        models.sort_by(|a, b| b.version.cmp(&a.version));
        let max = models.first().unwrap();
        map.insert(name, max.path.clone());
        for model in models {
            map.insert(format!("{}@{}", model.name, model.version), model.path);
        }
    }
    let imports = json! ({
         "imports": map
    });
    std::fs::write("./public/component-manifest.json", imports.to_string())?;
    Ok(())
}
pub async fn download_to_local(id: i32) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let model = entity::render_dependence::Entity::find_by_id(id)
        .one(db)
        .await?;
    let base_url = Url::parse(&std::env::var("BASE_URL").unwrap())?;

    if let Some(model) = model {
        if !model.path.starts_with("http") {
            //已下载
            return Ok(());
        }
        //下载
        ////创建文件
        let path = PathBuf::from("public/dep/")
            .join(format!("{}_{}.{}", &model.name, &model.version, &model.ext));

        let full_path = std::env::current_dir().unwrap().join(&path);
        if !full_path.parent().unwrap().exists() {
            std::fs::create_dir_all(full_path.parent().unwrap())?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(full_path)?;
        let mut bytes = reqwest::get(&model.path).await?.bytes_stream();
        while let Some(bytes) = bytes.next().await {
            let bytes = bytes?;
            file.write_all(&bytes)?;
        }
        file.flush()?;
        ////更新数据库
       let url= base_url.join(&path.to_str().unwrap().to_string())?;
        entity::render_dependence::Entity::update_many()
            .set(entity::render_dependence::ActiveModel {
                path: Set(url.to_string() ),
                ..Default::default()
            })
            .filter(entity::render_dependence::Column::Id.eq(id))
            .exec(db)
            .await?;
        rewrite_manifest().await?;
        Ok(())
    } else {
        anyhow::bail!("Dependence not found");
    }
}
