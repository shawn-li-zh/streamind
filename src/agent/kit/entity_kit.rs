use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::Local;

use entity::types::array::StringArray;
use sea_orm::{
    prelude::Expr,
    ActiveModelTrait,
    ActiveValue::{self, Set},
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, ExprTrait, QueryFilter, TransactionTrait,
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
        context::section::Section,
        event::{AgentEvent, AgentEventData},
        global::{AGENT_CTX_POOL, ITEM_GROUPS, TOOL_PACKAGE_TOOL},
    },
    global::DB,
};

pub async fn save_event(
    db: &impl ConnectionTrait,
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
        state_id: Set(agent_info.state_id),
        step: Set(agent_info.step),
        event_type: Set(event.data.type_enum()),
        force: Set(event.force),
        time: Set(event.time),
        payload: Set(event.data.json()),
        ..Default::default()
    };
    let model: entity::event::Model = event_model.insert(db).await?;
    Ok((event, model))
}

pub async fn save_section<T: Serialize>(
    agent_info: &AgentInfo,
    resp_event_id: i64,
    section: &Section,
    taken_time: u32,
    meta: T,
) -> anyhow::Result<entity::section::Model> {
    let json = serde_json::to_value(meta)?;

    let model = entity::section::ActiveModel {
        agent_id: Set(agent_info.id),
        state_id: Set(agent_info.state_id),
        event_id: Set(resp_event_id),
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
                        .exec_with_returning(txn)
                        .await?;
                    models.extend(arr);
                }
                //更新
                for model in update_models {
                    let model = model.update(txn).await?;
                    models.push(model);
                }
                //删除
                entity::item::Entity::delete_many()
                    .filter(entity::item::Column::AgentId.eq(agent_id))
                    .filter(entity::item::Column::Group.eq(group_name))
                    .filter(entity::item::Column::Name.is_in(removes))
                    .exec(txn)
                    .await?;
                Ok(models)
            })
        })
        .await?;

    Ok(())
}

pub async fn build_agent_context(model: entity::agent::Model) -> Arc<RwLock<AgentContext>> {
    let agent = AgentInfo {
        id: model.id,
        parent_id: model.parent_id,
        state_id: model.state_id,
        // state_pos: model.state_pos,
        step: model.step,
        name: model.name,
        model: model.model,
        workspace: model.workspace,
        avatar: model.avatar,
        status: AgentStatus::Idle,
    };
    //
    let tool_engine_context = {
        let mut tool_pkgs = HashSet::new();
        for pkg_name in &model.tool_packages.0 {
            if pkg_name == "*" {
                tool_pkgs.extend(TOOL_PACKAGE_TOOL.keys());
                break;
            }
            tool_pkgs.insert(TOOL_PACKAGE_TOOL.get(pkg_name.as_str()).expect("包不存在").name);
        }
        ToolEngineContext::build(tool_pkgs, &*TOOL_PACKAGE_TOOL)
    };
    let agent_context = AgentContext {
        info: agent,
        tool_engine_context: Arc::new(tool_engine_context),
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
    core_state: &str,
    tool_packages: Vec<String>,
) -> anyhow::Result<entity::agent::Model> {
    let database_connection = DB.get().unwrap();
    let agent = entity::agent::ActiveModel {
        name: Set(name),
        parent_id: Set(parent_id),
        step: Set(0),
        state_id: Set(0),
        model: Set(model),
        create_time: Set(Local::now()),
        workspace: Set("/".to_string()),
        avatar: Set("".to_string()),
        tool_packages: Set(StringArray(tool_packages)),
        ..Default::default()
    };
    let agent = agent.insert(database_connection).await?;
    push_state_stack(agent.id, 0, "核心记忆".to_string(), core_state.to_string()).await?;
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
