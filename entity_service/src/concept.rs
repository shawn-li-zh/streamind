use crate::DB;
use entity::{
    rule_concept::{ActiveModel, Column, Entity as Concept, Model},
    types::vector::Vector,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Statement,
};

/// 1. 增加：只增加名称和内容
pub async fn create(
    db: &impl ConnectionTrait,
    scope_id: i64,
    name: String,
    content: String,
) -> anyhow::Result<i64> {
    let model = ActiveModel {
        scope_id: Set(scope_id),
        name: Set(name),
        content: Set(content),
        embedded: Set(false),                 // 初始状态为未向量化
        embedding: Set(Some(Vector(vec![]))), // 初始向量为空
        create_time: Set(chrono::Local::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(model.id)
}

/// 2. 删除：按指定 id 删除
pub async fn delete(db: &impl ConnectionTrait, id: i64) -> anyhow::Result<()> {
    Concept::delete_many()
        .filter(Column::Id.eq(id))
        .exec(db)
        .await?;

    // 如果你有虚拟表，建议同步删除虚拟表内容
    db.execute_unprepared(&format!("DELETE FROM vec_concept WHERE id = {}", id))
        .await?;

    Ok(())
}

/// 3. 修改：修改指定 id 的向量值和状态
pub async fn update_vector(
    db: &impl ConnectionTrait,
    id: i64,
    embedding: Vec<f32>,
) -> anyhow::Result<()> {
    let vector_json = serde_json::to_string(&embedding)?;
    // a. 更新主表数据
    // 注意：这里假设你定义的 Vector 类型可以直接转换
    let vector_data = Vector(embedding);

    Concept::update_many()
        .set(entity::rule_concept::ActiveModel {
            embedding: Set(Some(vector_data)),
            embedded: Set(true),
            ..Default::default()
        })
        .filter(Column::Id.eq(id))
        .exec(db)
        .await?;

    // b. 同步更新/插入到 sqlite-vec 虚拟表 (vec_concept)
    // 使用 REPLACE 确保如果已存在则更新，不存在则插入

    let sql = "INSERT OR REPLACE INTO vec_concept(rowid, embedding) VALUES (?, ?)";
    let stem =
        Statement::from_sql_and_values(DbBackend::Sqlite, sql, vec![id.into(), vector_json.into()]);

    db.execute_raw(stem).await?;

    Ok(())
}

/// 4. 查询：按 name 唯一查询 (不需要传入 DB)
pub async fn find_by_name(name: &str, embedded: bool) -> anyhow::Result<Option<Model>> {
    let db = DB.get().expect("Database connection not initialized");
    let res = Concept::find()
        .filter(Column::Name.eq(name))
        .filter(Column::Embedded.eq(embedded))
        .one(db)
        .await?;
    Ok(res)
}
pub async fn find_by_names(scope_id: i64, names: &Vec<String>) -> anyhow::Result<Vec<Model>> {
    let db = DB.get().expect("Database connection not initialized");
    let res = Concept::find()
        .filter(Column::ScopeId.eq(scope_id))
        .filter(Column::Name.is_in(names))
        .all(db)
        .await?;
    Ok(res)
}
/// 5. 向量查询：按相似度获取 TopK (不需要传入 DB)
/// 返回匹配的模型及其距离
pub async fn search_similar(
    query_vector: Vec<f32>,
    top_k: u32,
) -> anyhow::Result<Vec<(Model, f64)>> {
    let db = DB.get().expect("Database connection not initialized");

    // a. 首先从虚拟表检索最相似的 ID 和距离
    let vector_json = serde_json::to_string(&query_vector)?;

    // sqlite-vec 的 KNN 查询语法
    let sql = r#"
        SELECT id, distance 
        FROM vec_concept 
        WHERE embedding MATCH ? 
          AND k = ?
        ORDER BY distance
    "#;

    // 定义一个临时结构体接收原始 SQL 结果
    #[derive(FromQueryResult)]
    struct QueryResult {
        id: i64,
        distance: f64,
    }

    let search_results = QueryResult::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        vec![vector_json.into(), top_k.into()],
    ))
    .all(db)
    .await?;

    if search_results.is_empty() {
        return Ok(vec![]);
    }

    // b. 根据返回的 ID 获取详细的模型数据
    let ids: Vec<i64> = search_results.iter().map(|r| r.id).collect();

    // 保持原始距离顺序获取模型
    let models = Concept::find()
        .filter(Column::Id.is_in(ids.clone()))
        .all(db)
        .await?;

    // 将模型与距离重新配对并按 ID 排序回原始检索顺序
    let mut final_res = Vec::new();
    for row in search_results {
        if let Some(m) = models.iter().find(|m| m.id == row.id) {
            final_res.push((m.clone(), row.distance));
        }
    }

    Ok(final_res)
}

pub async fn list_not_embedded(limit: u64) -> anyhow::Result<Vec<entity::rule_concept::Model>> {
    let db = DB.get().unwrap();
    let concepts: Vec<Model> = entity::rule_concept::Entity::find()
        .filter(entity::rule_concept::Column::Embedded.eq(false))
        .limit(limit)
        .all(db)
        .await?;
    Ok(concepts)
}

pub async fn page_concepts(
    page: usize,
    size: usize,
    mut key: Option<String>,
) -> anyhow::Result<Vec<entity::rule_concept::Model>> {
    key = key.filter(|s| !s.is_empty());
    let db = DB.get().unwrap();
    let mut select = entity::rule_concept::Entity::find();
    if let Some(key) = key {
        select = select.filter(entity::rule_concept::Column::Name.contains(&key));
    }
    let models = select
        .order_by_desc(entity::rule_concept::Column::Id)
        .offset((page.saturating_sub(1) * size) as u64)
        .limit(size as u64)
        .all(db)
        .await?;
    Ok(models)
}
