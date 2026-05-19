use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, query, sea_query::Expr,
};

use crate::DB;

pub async fn list(agent_id: u32) -> anyhow::Result<Vec<entity::agent_timer::Model>> {
    let db = DB.get().unwrap();
    let res = entity::agent_timer::Entity::find()
        .filter(entity::agent_timer::Column::AgentId.eq(agent_id))
        .filter(entity::agent_timer::Column::Disused.eq(false))
        .all(db)
        .await?;
    Ok(res)
}

pub async fn range(
    agent_id: u32,
    start: u64,
    num: u64,
    key: Option<String>,
) -> anyhow::Result<Vec<entity::agent_timer::Model>> {
    let db = DB.get().unwrap();
    let mut query = entity::agent_timer::Entity::find()
        .filter(entity::agent_timer::Column::AgentId.eq(agent_id))
        .filter(entity::agent_timer::Column::Disused.eq(false));
    if let Some(key) = key {
        query = query.filter(entity::agent_timer::Column::Payload.like(key));
    }
    let res = query
        .limit(num)
        .offset(start)
        .order_by_asc(entity::agent_timer::Column::Id)
        .all(db)
        .await?;
    Ok(res)
}

pub async fn update_timer(
    db: &DatabaseConnection,
    id: i64,
    corn: Option<String>,
    payload: Option<String>,
    times: Option<i32>,
) -> anyhow::Result<()> {
    entity::agent_timer::Entity::update_many()
        .col_expr(entity::agent_timer::Column::Corn, Expr::value(corn))
        .col_expr(entity::agent_timer::Column::Payload, Expr::value(payload))
        .col_expr(entity::agent_timer::Column::Times, Expr::value(times))
        .filter(entity::agent_timer::Column::Id.eq(id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn cancel(db: &DatabaseConnection, id: i64) -> anyhow::Result<()> {
    entity::agent_timer::Entity::update_many()
        .set(entity::agent_timer::ActiveModel {
            disused: Set(true),
            ..Default::default()
        })
        .filter(entity::agent_timer::Column::Id.eq(id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete(db: &DatabaseConnection, id: i64) -> anyhow::Result<()> {
    entity::agent_timer::Entity::delete_many()
        .filter(entity::agent_timer::Column::Id.eq(id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn create(
    db: &DatabaseConnection,
    agent_id: u32,
    corn: String,
    times: u32,
    payload: String,
) -> anyhow::Result<i64> {
    let model = entity::agent_timer::ActiveModel {
        agent_id: Set(agent_id),
        corn: Set(corn),
        payload: Set(payload),
        times: Set(times),
        trigger_times: Set(0),
        disused: Set(false),
        create_time: Set(chrono::Local::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(model.id)
}
