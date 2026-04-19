use std::{collections::HashSet, sync::{Arc, atomic::{AtomicI64, AtomicPtr, Ordering}}};

use chrono::Local;

use dashmap::DashMap;
use sea_orm::{
    prelude::Expr,
    ActiveModelTrait,
    ActiveValue::{self, Set},
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter,
};
use serde::Serialize;
use streamind_core::{
    agent::{AgentContext, AgentInfo, AgentStatus},
    tool::engine::ToolEngineContext,
};

use crate::{
    agent::{
        event::{AgentEvent, AgentEventData},
        global::{AGENT_CTX_POOL, TOOL_PACKAGE_TOOL},
        section::Section,
    },
    global::DB,
};

pub async fn save_event(
    agent_info: &AgentInfo,
    event_data: AgentEventData,
    force: bool,
) -> anyhow::Result<(AgentEvent, entity::event::Model)> {
    //保存数据库
    let event = AgentEvent {
        id: 0,
        agent_id: agent_info.id,
        data: event_data,
        force,
        time: Local::now(),
    };
    let event_model = entity::event::ActiveModel {
        agent_id: Set(event.agent_id),
        state_version: Set(agent_info.state_version.load(Ordering::Relaxed)),
        step: Set(agent_info.step.load(Ordering::Relaxed)),
        event_type: Set(event.data.type_enum()),
        force: Set(event.force),
        time: Set(event.time),
        payload: Set(event.data.json()),
        ..Default::default()
    };
    let model: entity::event::Model = event_model.insert(DB.get().unwrap()).await?;
    Ok((event, model))
}

pub async fn save_section<T: Serialize>(
    agent_info: &AgentInfo,
    section: &Section,
    taken_time: u32,
    meta: T,
) -> anyhow::Result<entity::section::Model> {
    let json = serde_json::to_value(meta)?;

    let model = entity::section::ActiveModel {
        agent_id: Set(agent_info.id),
        state_version: Set(agent_info.state_version.load(Ordering::Relaxed)),
        step: Set(agent_info.step.load(Ordering::Relaxed)),
        name: Set(section.tag.name.to_string()),
        taken_time: Set(taken_time),
        meta: Set(json),
        ..Default::default()
    };
    let model = model.insert(DB.get().unwrap()).await?;
    Ok(model)
}

pub async fn incr_agent_step(agent_id: u32, cur_step: i64) -> anyhow::Result<i64> {
    let db = DB.get().unwrap();
    let _ = entity::agent::Entity::update_many()
        .col_expr(
            entity::agent::Column::Step,
            Expr::col(entity::agent::Column::Step).add(1),
        ) // 版本自增
        .filter(entity::agent::Column::Id.eq(agent_id))
        .filter(entity::agent::Column::Step.eq(cur_step)) // 版本锁核心条件
        .exec(db)
        .await?;

    Ok(cur_step + 1)
}

pub async fn incr_agent_state_version(agent_id: u32, cur_version: i64) -> anyhow::Result<i64> {
    let db = DB.get().unwrap();
    let _ = entity::agent::Entity::update_many()
        .col_expr(
            entity::agent::Column::StateVersion,
            Expr::col(entity::agent::Column::StateVersion).add(1),
        ) // 版本自增
        .filter(entity::agent::Column::Id.eq(agent_id))
        .filter(entity::agent::Column::StateVersion.eq(cur_version)) // 版本锁核心条件
        .exec(db)
        .await?;

    Ok(cur_version + 1)
}

pub async fn build_agent_context(model: entity::agent::Model) -> Arc<AgentContext> {
    let mut status= AgentStatus::Idle;
    let agent = AgentInfo {
        id: model.id,
        parent_id: model.parent_id,
        state_version: AtomicI64::new(model.state_version),
        step: AtomicI64::new(model.step),
        name: model.name,
        workspace: model.workspace,
        avatar: model.avatar,
        status: AtomicPtr::new(&mut status),
    };
    //
    let tool_packages: HashSet<&str> = model
        .tool_packages
        .0
        .into_iter()
        .map(|v| TOOL_PACKAGE_TOOL.get(v.as_str()).unwrap().name)
        .collect();
    let tool_engine_context = ToolEngineContext::build(tool_packages, &*TOOL_PACKAGE_TOOL);
    let agent_context = AgentContext {
        info: agent,
        tool_engine_context,
    };
    //
    let id = agent_context.info.id;
    let mut p = AGENT_CTX_POOL.write().await;
    p.insert(id, Arc::new(agent_context));
    let agent_context = p.get(&id).unwrap();
    agent_context.tool_engine_context.init(agent_context.clone());

    agent_context.clone()
}
