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
/// 比较点分版本号：`1.10.0` 必须大于 `1.9.0`，
/// 直接用字符串比较会得出相反结论，从而选错「最新版本」
pub(crate) fn compare_version(a: &str, b: &str) -> std::cmp::Ordering {
    fn key(v: &str) -> Vec<u64> {
        v.split(['.', '-', '+', '_'])
            .map(|s| s.trim().parse::<u64>().unwrap_or(0))
            .collect()
    }
    let (ka, kb) = (key(a), key(b));
    for i in 0..ka.len().max(kb.len()) {
        let x = ka.get(i).copied().unwrap_or(0);
        let y = kb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
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
        models.sort_by(|a, b| compare_version(&b.version, &a.version));
        let max = models.first().unwrap();
        map.insert(name, max.path.clone());
        for model in models {
            map.insert(format!("{}@{}", model.name, model.version), model.path);
        }
    }
    let imports = json! ({
         "imports": map
    });
    // 首次运行 / 全新克隆时 public/ 可能不存在
    std::fs::create_dir_all("./public")?;
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
#[cfg(test)]
mod tests {
    use std::cmp::Ordering::*;

    use super::compare_version;

    #[test]
    fn version_compare_is_numeric_not_lexical() {
        // 字符串比较会把 "1.9.0" 判为更大
        assert_eq!(compare_version("1.10.0", "1.9.0"), Greater);
        assert_eq!(compare_version("1.9.0", "1.10.0"), Less);
        assert_eq!(compare_version("2.0.0", "10.0.0"), Less);
        assert_eq!(compare_version("3.2.25", "3.2.25"), Equal);
        // 段数不同时缺省段按 0 处理
        assert_eq!(compare_version("1.0", "1.0.0"), Equal);
        assert_eq!(compare_version("1.0.1", "1.0"), Greater);
        // 非数字段（预发布标识）按 0 处理，不应 panic
        assert_eq!(compare_version("1.0.0-beta", "1.0.0"), Equal);
    }
}
