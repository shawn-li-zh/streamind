use std::sync::Arc;

use entity_service::DB;
use futures::{FutureExt, StreamExt};
use kernel::parser;
use runtime::{
    agent::event::{AppEnvEvent, AppEnvEventData},
    context::stack::{Node, StackCommand},
};

use crate::{
    agent::{self, app::AppAgent},
    global::AGENT_POOL,
    plugin::memory,
};
pub fn parse_push(header: &str, content: &str) -> anyhow::Result<Node> {
    let inner = header
        .trim_start_matches("<push")
        .trim_end_matches(">")
        .trim();
    let mut attrs = parser::parse_attrs(inner);
    let title = attrs
        .remove("title")
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .ok_or_else(|| anyhow::anyhow!("push标签缺少title属性"))?;
    Ok(Node {
        id: 0,
        pos: 0,
        title: title.trim().to_string(),
        content: content.trim().to_string(),
    })
}

pub async fn handle_stack_commands(
    agent: Arc<AppAgent>,
    stack_cmds: Vec<StackCommand>,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let agent_id = agent.identity().id();
    let stack = agent.event_context();
    let backup_nodes: Vec<(Node, Vec<Arc<runtime::agent::event::AppEnvEvent>>)> = stack.backup();
    let mut need_memory = false;
    for cmd in stack_cmds {
        match cmd {
            StackCommand::Push(mut node) => {
                //设置节点的位置
                {
                    let pos = stack.top_node().map(|v| v.pos).unwrap_or_default();
                    node.pos = pos + 1;
                }
                let model = entity_service::state::push(
                    db,
                    agent_id,
                    node.pos as _,
                    node.title.clone(),
                    node.content.clone(),
                )
                .await?;
                node.id = model.id;
                stack.push_node(node);
                println!("push node to stack");
            }
            StackCommand::Reset(content) => {
                need_memory = true;
                if let Some(top_node) = stack.top_node() {
                    let node_id = top_node.id;
                    let c = content.clone();
                    entity_service::transaction(|db| {
                        async move {
                            entity_service::event::unload_events(db, node_id).await?;
                            entity_service::state::reset(db, node_id, c).await?;
                            entity_service::rule::inactivate_rules(db, node_id).await?;
                            Ok(())
                        }
                        .boxed()
                    })
                    .await?;
                    // 用 reset_top_node 而不是 top_node.reset，把体积统计一起扣掉
                    stack.reset_top_node(content);
                } else {
                    anyhow::bail!("栈顶不存在")
                }
                println!("reset node");
            }
            StackCommand::Pop(result) => {
                if stack.node_len() <= 1 {
                    anyhow::bail!("核心栈状态不能弹出")
                }
                need_memory = true;
                let last_top_node = stack.pop_node().unwrap();
                let title = &last_top_node.title;
                let node_id = last_top_node.id;
                let newly_top_node_id = stack.top_node().map(|v| v.id).unwrap_or(1);
                entity_service::transaction(|db| {
                    async move {
                        entity_service::event::unload_events(db, node_id).await?;
                        entity_service::state::pop(db, agent_id, node_id, newly_top_node_id)
                            .await?;
                        entity_service::rule::inactivate_rules(db, node_id).await?;
                        Ok(())
                    }
                    .boxed()
                })
                .await?;

                if let Some(top_node) = stack.top_node() {
                    let event = AppEnvEventData::StackPop(title.clone(), result);
                    let event = agent.save_event((event, false)).await?;
                    top_node.push(event);
                } else {
                    anyhow::bail!("栈顶不存在")
                }
                println!("pop node from stack");
            }
        }
    }
    if need_memory {
        let items = backup_nodes
            .into_iter()
            .map(|(node, events)| memory::TaskDataItem {
                node,
                event_ids: events.into_iter().map(|v| v.id).collect(),
            })
            .collect();
        agent.memory_agent.push_task(items).await?;
    }

    Ok(())
}
