use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Local};

use entity::{agent::AgentStatus, types::array::StringArray};
use kernel::simple_id;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{self, Set},
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
};

use crate::DB;

pub static CORE_STATE: &str = include_str!("../../prompts/core_state.md");
pub async fn save(
    db: &impl ConnectionTrait,
    name: String,
    model_id: u32,
    parent_id: u32,
    avatar: String,
    persona: String,
    tool_packages: Vec<String>,
) -> anyhow::Result<entity::agent::Model> {
    let code = simple_id(6);
    //
    let agent = entity::agent::ActiveModel {
        name: Set(name),
        parent_id: Set(parent_id),
        state_id: Set(0),
        model_id: Set(model_id),
        avatar: Set(avatar),
        workspace: Set(format!("workspace/{}", &code)),
        persona: Set(persona),
        code: Set(code),
        tool_packages: Set(StringArray(tool_packages)),
        create_time: Set(Local::now()),
        status: Set(AgentStatus::Idle),
        ..Default::default()
    };
    let agent = agent.insert(db).await?;
    //
    let state = format!("{CORE_STATE}");
    crate::state::push(db, agent.id, 0, "根节点".to_string(), state).await?;
    Ok(agent)
}

pub async fn update_status(
    db: &impl ConnectionTrait,
    agent_id: u32,
    status: kernel::agent::context::AgentStatus,
) -> anyhow::Result<()> {
    let s = serde_json::to_string(&status)?;
    let status: AgentStatus = serde_json::from_str(&s)?;
    let agent = entity::agent::ActiveModel {
        status: Set(status),

        ..Default::default()
    };
    entity::agent::Entity::update_many()
        .set(agent)
        .filter(entity::agent::Column::Id.eq(agent_id))
        .exec(db)
        .await?;

    Ok(())
}
pub async fn startup(db: &impl ConnectionTrait, agent_id: u32) -> anyhow::Result<()> {
    entity::agent::Entity::update_many()
        .set(entity::agent::ActiveModel {
            startup_time: Set(Some(Local::now())),
            ..Default::default()
        })
        .filter(entity::agent::Column::Id.eq(agent_id))
        .filter(entity::agent::Column::Id.eq(agent_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn get_by_id(id: u32) -> anyhow::Result<Option<entity::agent::Model>> {
    let db = DB.get().unwrap();
    let agent = entity::agent::Entity::find_by_id(id).one(db).await?;
    Ok(agent)
}

pub async fn tree() -> anyhow::Result<Vec<entity::agent::Model>> {
    let db = DB.get().unwrap();
    let agents = entity::agent::Entity::find()
        .order_by_asc(entity::agent::Column::ParentId)
        .all(db)
        .await
        .unwrap();
    let mut id_map_agent = HashMap::new();
    for agent in agents {
        if agent.parent_id == 0 {
            id_map_agent.insert(agent.id, agent);
            continue;
        }
        if let Some(parent) = id_map_agent.get_mut(&agent.parent_id) {
            parent.children.push(agent);
        }
    }
    Ok(id_map_agent.into_values().collect())
}

pub async fn list_by_model_ids(model_ids: Vec<u32>) -> anyhow::Result<Vec<entity::agent::Model>> {
    let db = DB.get().unwrap();
    let agents = entity::agent::Entity::find()
        .filter(entity::agent::Column::ModelId.is_in(model_ids))
        .order_by_asc(entity::agent::Column::ParentId)
        .all(db)
        .await
        .unwrap();
    Ok(agents)
}

pub async fn update_rule_groups(
    db: &impl ConnectionTrait,
    agent_id: u32,
    rule_groups: Vec<String>,
) -> anyhow::Result<()> {
    let agent = entity::agent::ActiveModel {
        rule_groups: Set(StringArray(rule_groups)),
        ..Default::default()
    };
    entity::agent::Entity::update_many()
        .set(agent)
        .filter(entity::agent::Column::Id.eq(agent_id))
        .exec(db)
        .await?;
    Ok(())
}
