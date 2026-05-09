use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use chrono::Local;

use dashmap::DashMap;
use entity::{agent, event::EnvEventType, types::array::StringArray};
use sea_orm::{
    prelude::Expr,
    ActiveModelTrait,
    ActiveValue::{self, Set},
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, ExprTrait, QueryFilter, TransactionTrait,
};

use serde::Serialize;
use streamind_core::{
    agent::{AgentContext, AgentInfo, AgentStatus, PermissionLife},
    event::{EnvEvent, ToolCallDetailItem},
    item::{Item, ItemGroup},
    render::RenderContext,
    section::Section,
    simple_id,
    tool::engine::ToolEngineContext,
};
use tokio::sync::RwLock;

use crate::{
    agent::{
        event::{to_json, to_type_enum, AgentEvent},
        global::{AGENT_CTX_POOL, ENGINE_POOL, TOOL_PACKAGE_TOOL},
    },
    global::DB,
};

pub async fn save_event(
    db: &impl ConnectionTrait,
    agent_info: &AgentInfo,
    event_data: EnvEvent,
    force: bool,
) -> anyhow::Result<(AgentEvent, entity::event::Model)> {
    //保存数据库
    let mut event = AgentEvent {
        id: 0,
        agent_id: agent_info.id,
        data: event_data.clone(),
        force,
        time: Local::now(),
    };
    let event_type = to_type_enum(&event_data);
    let json = to_json(event_data);
    let event_model = entity::event::ActiveModel {
        agent_id: Set(event.agent_id),
        state_id: Set(agent_info.state_id),
        step: Set(0),
        event_type: Set(event_type),
        force: Set(event.force),
        time: Set(event.time),
        data: Set(json),
        ..Default::default()
    };
    let model: entity::event::Model = event_model.insert(db).await?;
    event.id = model.id;
    Ok((event, model))
}

pub async fn update_tool_call_result(
    event_id: i64,
    details: Vec<ToolCallDetailItem>,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    //更新section
    for ele in &details {
        update_section_payload(ele.id, ele.clone()).await?;
    }
    //
    let data = serde_json::to_value(details)?;
    let model = entity::event::ActiveModel {
        data: Set(data),
        ..Default::default()
    };

    let result = entity::event::Entity::update_many()
        .set(model)
        .filter(entity::event::Column::Id.eq(event_id))
        .filter(entity::event::Column::EventType.eq(EnvEventType::ToolCallResult))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}

pub async fn update_agent_response(
    event_id: i64,
    event_type: EnvEventType,
    sections: Vec<Section>,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    if sections.is_empty() {
        //删除
        let result = entity::event::Entity::delete_many()
            .filter(entity::event::Column::Id.eq(event_id))
            .filter(entity::event::Column::EventType.eq(event_type))
            .exec(db)
            .await?;
        return if result.rows_affected == 1 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("更新失败"))
        };
    }
    let data = serde_json::to_value(sections)?;
    let model = entity::event::ActiveModel {
        data: Set(data),
        ..Default::default()
    };
    let result = entity::event::Entity::update_many()
        .set(model)
        .filter(entity::event::Column::Id.eq(event_id))
        .filter(entity::event::Column::EventType.eq(event_type))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}

pub async fn push_state_stack(
    agent_id: u32,
    pos: u32,
    title: String,
    content: String,
) -> anyhow::Result<entity::state::Model> {
    let db = DB.get().unwrap();
    let model = db
        .transaction::<_, _, DbErr>(|txn| {
            Box::pin(async move {
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
                let model = state.insert(txn).await?;
                //指向最新状态
                let _ = entity::agent::Entity::update_many()
                    .col_expr(
                        entity::agent::Column::StateId,
                        Expr::col(entity::agent::Column::StateId).eq(model.id),
                    )
                    .filter(entity::agent::Column::Id.eq(agent_id))
                    .exec(txn)
                    .await?;
                Ok(model)
            })
        })
        .await?;
    Ok(model)
}



pub async fn startup_agent(agent_id: u32) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
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

pub static CORE_STATE: &str = include_str!("../../../prompts/core_state.md");
pub async fn new_agent(
    name: String,
    model_id: u32,
    parent_id: u32,
    avatar: String,
    persona: String,
    tool_packages: Vec<String>,
) -> anyhow::Result<entity::agent::Model> {
    let code = simple_id(9);
    //
    let db = DB.get().unwrap();
    let agent = entity::agent::ActiveModel {
        name: Set(name),
        parent_id: Set(parent_id),
        state_id: Set(0),
        model_id: Set(model_id),
        avatar: Set(avatar),
        workspace: Set(format!("/workspace/{}", &code)),
        persona: Set(persona),
        code: Set(code),
        tool_packages: Set(StringArray(tool_packages)),
        create_time: Set(Local::now()),
        ..Default::default()
    };
    let agent = agent.insert(db).await?;
    //
    let state = format!("{CORE_STATE}");
    push_state_stack(agent.id, 0, "核心记忆".to_string(), state).await?;
    Ok(agent)
}

pub async fn dump_states(agent_id: u32, state_ids: Vec<i64>) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    entity::state::Entity::update_many()
        .col_expr(
            entity::state::Column::Dumped,
            Expr::col(entity::state::Column::Dumped).eq(true),
        )
        .filter(entity::state::Column::AgentId.eq(agent_id))
        .filter(entity::state::Column::Id.is_in(state_ids))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn save_section(
    agent_id: u32,
    event_id: i64,
    section: Section,
    payload: serde_json::Value,
) -> anyhow::Result<entity::section::Model> {
    let db = DB.get().unwrap();
    let model = entity::section::ActiveModel {
        agent_id: Set(agent_id),
        event_id: Set(event_id),
        name: Set(section.tag_name),
        content: Set(section.content),
        payload: Set(payload),
        time: Set(section.time),
        ..Default::default()
    };
    let model = model.insert(db).await?;
    Ok(model)
}

pub async fn update_section_payload<T: Serialize>(id: i64, payload: T) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let model = entity::section::ActiveModel {
        payload: Set(serde_json::to_value(payload)?),
        ..Default::default()
    };
    let result = entity::section::Entity::update_many()
        .set(model)
        .filter(entity::section::Column::Id.eq(id))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(anyhow::anyhow!("更新失败"));
    }
    Ok(())
}
