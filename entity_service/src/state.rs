use chrono::Local;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait as _, ConnectionTrait, EntityTrait, ExprTrait,
    QueryFilter, QueryOrder, prelude::Expr,
};

use crate::DB;

pub async fn push(
    db: &impl ConnectionTrait,
    agent_id: u32,
    pos: u32,
    title: String,
    content: String,
) -> anyhow::Result<entity::state::Model> {
    //创建历史状态
    let state = entity::state::ActiveModel {
        agent_id: Set(agent_id),
        pos: Set(pos),
        dumped: Set(false),
        title: Set(title),
        content: Set(content),
        time: Set(Local::now()),
        ..Default::default()
    };
    let model = state.insert(db).await?;
    //指向最新状态
    let _ = entity::agent::Entity::update_many()
        .set(entity::agent::ActiveModel {
            state_id: Set(model.id),
            ..Default::default()
        })
        .filter(entity::agent::Column::Id.eq(agent_id))
        .exec(db)
        .await?;
    Ok(model)
}

// pub async fn dumps(
//     db: &impl ConnectionTrait,
//     agent_id: u32,
//     state_ids: Vec<i64>,
// ) -> anyhow::Result<()> {
//     entity::state::Entity::update_many()
//         .set(entity::state::ActiveModel {
//             dumped: Set(true),
//             ..Default::default()
//         })
//         .filter(entity::state::Column::AgentId.eq(agent_id))
//         .filter(entity::state::Column::Id.is_in(state_ids))
//         .exec(db)
//         .await?;
//     Ok(())
// }
// pub async fn dump(db: &impl ConnectionTrait, state_id: i64) -> anyhow::Result<()> {
//     entity::state::Entity::update_many()
//         .set(entity::state::ActiveModel {
//             dumped: Set(true),
//             ..Default::default()
//         })
//         .filter(entity::state::Column::Id.eq(state_id))
//         .exec(db)
//         .await?;
//     Ok(())
// }

pub async fn pop(
    db: &impl ConnectionTrait,
    agent_id: u32,
    state_id: i64,
    last_state_id: i64,
) -> anyhow::Result<()> {
    //卸载节点
    entity::state::Entity::update_many()
        .set(entity::state::ActiveModel {
            dumped: Set(true),
            ..Default::default()
        })
        .filter(entity::state::Column::Id.eq(state_id))
        .exec(db)
        .await?;

    //指向最新状态
    let _ = entity::agent::Entity::update_many()
        .set(entity::agent::ActiveModel {
            state_id: Set(last_state_id),
            ..Default::default()
        })
        .filter(entity::agent::Column::Id.eq(agent_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn reset(
    db: &impl ConnectionTrait,
    state_id: i64,
    content: String,
) -> anyhow::Result<()> {
    entity::state::Entity::update_many()
        .set(entity::state::ActiveModel {
            content: Set(content),
            ..Default::default()
        })
        .filter(entity::state::Column::Id.eq(state_id))
        .exec(db)
        .await?;
    Ok(())
}
pub async fn list(agent_id: u32) -> anyhow::Result<Vec<entity::state::Model>> {
    let db = DB.get().unwrap();
    let states = entity::state::Entity::find()
        .filter(entity::state::Column::AgentId.eq(agent_id))
        .filter(entity::state::Column::Dumped.eq(false))
        .order_by_asc(entity::state::Column::Pos)
        .all(db)
        .await?;
    Ok(states)
}
