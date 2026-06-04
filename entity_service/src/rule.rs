use std::{collections::HashMap, process::id};

use chrono::{DateTime, Local};
use entity::types::vector::Vector;
use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, ExprTrait, FromQueryResult,
    QueryFilter as _, QueryOrder as _, QuerySelect as _, Statement, raw_sql,
    sea_query::{Expr, OnConflict, Query, SelectStatement},
};
use serde::de;

use crate::DB;

pub async fn new_scope(
    db: &impl ConnectionTrait,
    name: String,
    content: String,
    active_threshold: f32,
) -> anyhow::Result<i64> {
    // let db = DB.get().unwrap();
    let scope = entity::rule_scope::ActiveModel {
        name: Set(name),
        content: Set(content),
        embedded: Set(false),
        embedding: Set(Vector(vec![])),
        active_threshold: Set(active_threshold),
        create_time: Set(Local::now()),
        ..Default::default()
    };
    let res: sea_orm::InsertResult<entity::rule_scope::ActiveModel> =
        entity::rule_scope::Entity::insert(scope).exec(db).await?;
    Ok(res.last_insert_id)
}

/// 3. 修改：修改指定 id 的向量值和状态
pub async fn update_scope_vector(
    db: &impl ConnectionTrait,
    id: i64,
    embedding: Vec<f32>,
) -> anyhow::Result<()> {
    let vector_json = serde_json::to_string(&embedding)?;
    let vector_data = Vector(embedding);

    entity::rule_scope::Entity::update_many()
        .set(entity::rule_scope::ActiveModel {
            embedding: Set(vector_data),
            embedded: Set(true),
            ..Default::default()
        })
        .filter(entity::rule_scope::Column::Id.eq(id))
        .exec(db)
        .await?;

    let sql = "INSERT OR REPLACE INTO vec_scope(rowid, embedding) VALUES (?, ?)";
    let stem =
        Statement::from_sql_and_values(DbBackend::Sqlite, sql, vec![id.into(), vector_json.into()]);

    db.execute_raw(stem).await?;

    Ok(())
}

pub async fn list_scopes_not_embedded(
    limit: u64,
) -> anyhow::Result<Vec<entity::rule_scope::Model>> {
    let db = DB.get().unwrap();
    let scopes: Vec<_> = entity::rule_scope::Entity::find()
        .filter(entity::rule_scope::Column::Embedded.eq(false))
        .limit(limit)
        .all(db)
        .await?;
    Ok(scopes)
}

pub async fn save_rule(
    db: &impl ConnectionTrait,
    scope_id: i64,
    name: String,
    expr: String,
    content: String,
    expire_time: DateTime<Local>,
) -> anyhow::Result<i64> {
    let rule = entity::rule_item::ActiveModel {
        scope_id: Set(scope_id),
        name: Set(name),
        expr: Set(expr),
        content: Set(content),
        expire_time: Set(expire_time),
        recently_time: Set(Local::now()),
        create_time: Set(Local::now()),
        ..Default::default()
    };
    let model = entity::rule_item::Entity::insert(rule)
        .on_conflict(
            OnConflict::columns([
                entity::rule_item::Column::ScopeId,
                entity::rule_item::Column::Name,
            ]) // 指定产生冲突的唯一键列
            .update_columns([
                entity::rule_item::Column::Expr,
                entity::rule_item::Column::Content,
                entity::rule_item::Column::ExpireTime,
                entity::rule_item::Column::CreateTime,
            ]) // 冲突时覆盖这些列
            .to_owned(),
        )
        .exec_with_returning(db)
        .await?;
    Ok(model.id)
}

pub async fn list_groups_by_names(
    group_names: &Vec<String>,
) -> anyhow::Result<Vec<entity::rule_scope::Model>> {
    let db = DB.get().unwrap();
    let models = entity::rule_scope::Entity::find()
        .filter(entity::rule_scope::Column::Name.is_in(group_names))
        .all(db)
        .await?;
    Ok(models)
}
pub async fn list_rules_by_group_names(
    group_names: &Vec<String>,
) -> anyhow::Result<Vec<entity::rule_item::Model>> {
    if group_names.is_empty() {
        return Ok(vec![]);
    }
    let db = DB.get().unwrap();
    let mut subquery: SelectStatement = SelectStatement::new();
    subquery
        .column(entity::rule_scope::Column::Id)
        .from(entity::rule_scope::Entity)
        .and_where(entity::rule_scope::Column::Name.is_in(group_names));

    let rules = entity::rule_item::Entity::find()
        .filter(entity::rule_item::Column::ScopeId.in_subquery(subquery))
        .all(db)
        .await?;
    Ok(rules)
}
fn vector_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|&f| f.to_le_bytes().to_vec()).collect()
}
pub async fn page_scopes(
    page: u64,
    size: u64,
    mut key: Option<String>,
    mut query_vector: Option<Vec<f32>>,
) -> anyhow::Result<Vec<entity::rule_scope::Model>> {
    key = key.filter(|s| !s.is_empty());
    query_vector = query_vector.filter(|v| !v.is_empty());
    let db = DB.get().unwrap();
    let mut select = entity::rule_scope::Entity::find();
    if let Some(key) = key {
        select = select.filter(
            entity::rule_scope::Column::Name
                .contains(&key)
                .or(entity::rule_scope::Column::Content.contains(&key)),
        );
    }
    let mut distances = HashMap::new();

    if let Some(query_vector) = query_vector {
        let blob_vector = vector_to_blob(&query_vector);
        let sql = r#"
      SELECT r.id,distance
FROM vec_scope v
JOIN rule_scope r ON v.rowid = r.id
WHERE v.embedding MATCH ? -- 利用 vec0 的索引进行 KNN 搜索
  AND k = 100              -- 先取出前100个最相似的
  AND vec_distance_cosine(v.embedding, ?) < (1.0 - r.active_threshold) -- 再过滤阈值
  order by distance
    "#;

        // 定义一个临时结构体接收原始 SQL 结果
        #[derive(FromQueryResult)]
        struct QueryResult {
            id: i64,
            distance: f64,
        }
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![
                blob_vector.clone().into(), // 用于 WHERE
                blob_vector.into(),         // 用于 ORDER BY
            ],
        );
        let search_results = QueryResult::find_by_statement(stmt).all(db).await?;
        // b. 根据返回的 ID 获取详细的模型数据
        let mut ids: Vec<i64> = vec![];
        for ele in search_results {
            distances.insert(ele.id, ele.distance);
            ids.push(ele.id);
        }
        if !ids.is_empty() {
            select = select.filter(entity::rule_scope::Column::Id.is_in(ids));
        }
    }
    let mut groups = select.offset((page - 1) * size).limit(size).all(db).await?;
    if groups.len() > distances.len() && distances.len() > 0 {
        groups.sort_by(|a, b| {
            let a = distances.get(&a.id).unwrap_or(&f64::MAX);
            let b = distances.get(&b.id).unwrap_or(&f64::MAX);
            a.partial_cmp(b).unwrap()
        });
    }
    Ok(groups)
}

pub async fn page_rule_in_scope(
    scope_id: i64,
    page: u64,
    size: u64,
    mut key: Option<String>,
) -> anyhow::Result<Vec<entity::rule_item::Model>> {
    key = key.filter(|s| !s.is_empty());
    let db = DB.get().unwrap();
    let mut select =
        entity::rule_item::Entity::find().filter(entity::rule_item::Column::ScopeId.eq(scope_id));
    if let Some(key) = key {
        select = select.filter(
            entity::rule_item::Column::Name
                .contains(&key)
                .or(entity::rule_item::Column::Expr.contains(&key))
                .or(entity::rule_item::Column::Content.contains(&key)),
        );
    }
    let rules = select
        .order_by_asc(entity::rule_item::Column::Id)
        .offset((page - 1) * size)
        .limit(size)
        .all(db)
        .await?;
    Ok(rules)
}

pub async fn list_not_active_rules(
    scope_ids: Vec<i64>,
) -> anyhow::Result<Vec<entity::rule_item::Model>> {
    let db = DB.get().unwrap();
    let rules = entity::rule_item::Entity::find()
        .filter(entity::rule_item::Column::ScopeId.is_in(scope_ids))
        .filter(entity::rule_item::Column::StateId.eq(0))
        .all(db)
        .await?;
    Ok(rules)
}

pub async fn inactivate_rules(db: &impl ConnectionTrait, state_id: i64) -> anyhow::Result<()> {
    entity::rule_item::Entity::update_many()
        .col_expr(entity::rule_item::Column::StateId, Expr::value(0))
        .filter(entity::rule_item::Column::StateId.eq(state_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn activate_rules(
    db: &impl ConnectionTrait,
    state_id: i64,
    rule_ids: Vec<i64>,
) -> anyhow::Result<()> {
    entity::rule_item::Entity::update_many()
        .col_expr(entity::rule_item::Column::StateId, Expr::value(state_id))
        .col_expr(
            entity::rule_item::Column::TriggerTimes,
            Expr::column(entity::rule_item::Column::TriggerTimes).add(1),
        )
        .col_expr(
            entity::rule_item::Column::RecentlyTime,
            Expr::value(Local::now()),
        )
        .filter(entity::rule_item::Column::Id.is_in(rule_ids))
        .exec(db)
        .await?;
    Ok(())
}
