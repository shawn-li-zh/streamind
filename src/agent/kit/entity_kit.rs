use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicI64, AtomicPtr, Ordering},
        Arc,
    },
};

use chrono::Local;

use dashmap::DashMap;
use sea_orm::{
    prelude::Expr,
    ActiveModelTrait,
    ActiveValue::{self, Set},
    ColumnTrait, DbErr, EntityTrait, ExprTrait, QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use streamind_core::{
    agent::{AgentContext, AgentInfo, AgentStatus},
    item::{Item, ItemGroup},
    tool::engine::ToolEngineContext,
};
use tokio::sync::RwLock;

use crate::{
    agent::{
        event::{AgentEvent, AgentEventData},
        global::{AGENT_CTX_POOL, BASE_ITEMS, ITEM_GROUPS, TOOL_PACKAGE_TOOL},
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
        state_version: Set(agent_info.state_version),
        step: Set(agent_info.step),
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
        state_version: Set(agent_info.state_version),
        step: Set(agent_info.step),
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

pub async fn incr_agent_state_version(
    agent_id: u32,
    cur_version: i64,
    title: String,
    summary: String,
) -> anyhow::Result<i64> {
    let db = DB.get().unwrap();
    //创建历史状态
    let state = entity::state::ActiveModel {
        agent_id: Set(agent_id),
        state_version: Set(cur_version),
        title: Set(title),
        summary: Set(summary),
        time: Set(Local::now()),
        ..Default::default()
    };
    state.insert(db).await?;
    //自增版本
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

pub async fn load_item_groups(agent_id: u32) -> Vec<ItemGroup> {
    let db = DB.get().unwrap();
    let items = entity::item::Entity::find()
        .filter(entity::item::Column::AgentId.eq(agent_id))
        .all(db)
        .await
        .unwrap();
    let mut map: HashMap<String, HashMap<String, Item>> = HashMap::new();
    for item in items {
        let g = item.group;
        let item = Item {
            id: item.id,
            name: item.name,
            content: item.content,
            editable: item.editable,
            removable: item.removable,
        };
        let items = map.entry(g).or_default();
        items.insert(item.name.to_string(), item);
    }
    let mut groups = vec![];
    for (name, desc) in &*ITEM_GROUPS {
        let items = map.remove(*name).unwrap_or_default();
        let g = ItemGroup {
            name,
            description: desc,
            items,
        };
        groups.push(g);
    }
    groups
}

pub async fn update_item_group(
    agent_id: u32,
    group: &mut ItemGroup,
    (saves, removes): (Vec<(String, String)>, Vec<String>),
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let mut save_models = vec![];
    let mut update_models = vec![];

    for (name, content) in saves {
        let item = group
            .items
            .entry(name.to_string())
            .or_insert_with(move || Item {
                id: 0,
                name: name,
                content: "".to_string(),
                editable: true,
                removable: true,
            });
        item.content = content;
        let mut model = entity::item::ActiveModel {
            agent_id: Set(agent_id),
            group: Set(group.name.to_string()),
            name: Set(item.name.to_string()),
            content: Set(item.content.to_string()),
            editable: Set(item.editable),
            removable: Set(item.removable),
            ..Default::default()
        };
        if item.id != 0 {
            model.id = Set(item.id);
            update_models.push(model);
        } else {
            save_models.push(model);
        }
    }
    //
    for name in &removes {
        group.items.remove(name);
    }
    let group_name = group.name;
    let models = db
        .transaction::<_, Vec<entity::item::Model>, DbErr>(|txn| {
            Box::pin(async move {
                //保存
                let mut models = vec![];
                if !save_models.is_empty() {
                    let arr = entity::item::Entity::insert_many(save_models)
                        .exec_with_returning(db)
                        .await?;
                    models.extend(arr);
                }
                //更新
                for model in update_models {
                    let model = model.update(db).await?;
                    models.push(model);
                }
                //删除
                entity::item::Entity::delete_many()
                    .filter(entity::item::Column::AgentId.eq(agent_id))
                    .filter(entity::item::Column::Group.eq(group_name))
                    .filter(entity::item::Column::Name.is_in(removes))
                    .exec(db)
                    .await?;
                Ok(models)
            })
        })
        .await?;

    Ok(())
}

pub async fn build_agent_context(
    model: entity::agent::Model,
    item_groups: Vec<ItemGroup>,
) -> Arc<RwLock<AgentContext>> {
    let agent = AgentInfo {
        id: model.id,
        parent_id: model.parent_id,
        state_version: model.state_version,
        step: model.step,
        name: model.name,
        model: model.model,
        workspace: model.workspace,
        avatar: model.avatar,
        status: AgentStatus::Idle,
    };
    //
    let tool_engine_context = {
        let tool_packages: HashSet<&str> = model
            .tool_packages
            .0
            .into_iter()
            .map(|v| TOOL_PACKAGE_TOOL.get(v.as_str()).unwrap().name)
            .collect();
        ToolEngineContext::build(tool_packages, &*TOOL_PACKAGE_TOOL)
    };
    let agent_context = AgentContext {
        info: agent,
        item_groups,
        tool_engine_context,
    };
    //
    let id = agent_context.info.id;
    agent_context.tool_engine_context.init(&agent_context);
    let agent_context = Arc::new(RwLock::new(agent_context));
    //
    let mut p = AGENT_CTX_POOL.write().await;
    p.insert(id, agent_context.clone());
    agent_context
}

pub async fn new_agent(
    name: String,
    model: String,
    parent_id: u32,
    items: HashMap<String, Vec<Item>>,
) -> anyhow::Result<entity::agent::Model> {
    let database_connection = DB.get().unwrap();
    let agent = entity::agent::ActiveModel {
        name: Set(name),
        parent_id: Set(parent_id),
        step: Set(0),
        state_version: Set(0),
        model: Set(model),
        create_time: Set(Local::now()),
        workspace: Set("/".to_string()),
        avatar: Set("".to_string()),
        ..Default::default()
    };
    let agent = agent.insert(database_connection).await?;
    //保存记忆条目
    let mut saved_items = vec![];
    for (group, items) in items {
        for item in items {
            let model = entity::item::ActiveModel {
                agent_id: Set(agent.id),
                group: Set(group.clone()),
                name: Set(item.name),
                content: Set(item.content),
                editable: Set(item.editable),
                ..Default::default()
            };
            saved_items.push(model);
        }
    }
    entity::item::Entity::insert_many(saved_items)
        .exec(database_connection)
        .await?;
    Ok(agent)
}
