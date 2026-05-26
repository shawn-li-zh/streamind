use std::{collections::HashMap, fs::File, sync::Arc};

use entity::event::EnvEventType;
use kernel::{
    agent::{context::AgentContext, AgentIdentity},
    client::ChatClient,
    tool::engine::ToolEngineContext,
};
use runtime::agent::{
    event::AppEnvEvent,
    event_context::{MindNode, Node, StackContext},
};

use crate::event::convert;

pub async fn load_stack(
    identity: &AgentIdentity,
    client: Arc<dyn ChatClient>,
    ctx: &AgentContext,
) -> anyhow::Result<StackContext<AppEnvEvent>> {
    let (agent_id, workspace) = { (identity.id(), identity.workspace()) };
    //查状态栈
    let states = entity_service::state::list(agent_id).await?;
    //查事件
    let state_ids: Vec<i64> = states.iter().map(|s: &entity::state::Model| s.id).collect();
    let events = entity_service::event::list_by_state_ids(agent_id, state_ids).await?;

    //查回复
    let mut response_event_ids: Vec<i64> = vec![];
    let mut state_id_map_events: HashMap<i64, Vec<entity::event::Model>> = HashMap::new();
    for e in events {
        if let EnvEventType::AgentResponse = &e.event_type {
            response_event_ids.push(e.id);
        }
        let events = state_id_map_events.entry(e.state_id).or_default();
        events.push(e);
    }
    let sections = entity_service::section::list_by_event_ids(agent_id, response_event_ids).await?;

    //
    let mut event_id_map_sections: HashMap<i64, Vec<entity::section::Model>> = HashMap::new();
    for s in sections {
        let arr = event_id_map_sections.entry(s.event_id).or_default();
        arr.push(s);
    }
    //
    let sp = build_system_prompt(identity, ctx);
    let stack = StackContext::new(client, sp);
    for state in states {
        let node = MindNode {
            node: Node {
                id: state.id,
                pos: state.pos as _,
                title: state.title,
                content: state.content,
            },
            events: vec![],
            content_len: 0,
        };
        stack.push_node(node);
        //
        let events = state_id_map_events.remove(&state.id).unwrap_or_default();
        for event in events {
            let agent_event = convert::from_entity(event);
            stack.push_event(agent_event);
        }
        //
    }
    //
    Ok(stack)
}
pub static EVENT_DRIVEN: &str = include_str!("../prompts/common/event_driven.md");
pub static GENERATE_RULE: &str = include_str!("../prompts/common/generate_rule.md");

pub static STACK_CONTEXT_RULE: &str = include_str!("../prompts/common/stack_context_rule.md");
pub static TOOL_CALL_RULE: &str = include_str!("../prompts/common/tool_call_rule.md");
pub static RENDER_RULE: &str = include_str!("../prompts/common/render_rule.md");

pub fn build_system_prompt(identity: &AgentIdentity, ctx: &AgentContext) -> String {
    let tool_engine_context = ctx.get::<ToolEngineContext>().unwrap();
    let items: Vec<String> = tool_engine_context
        .tool_packages
        .iter()
        .map(|pkg_name| {
            let pkg = tool_engine_context.package_pool.get(pkg_name).unwrap();
            let tool_size = pkg.tools().len();
            format!("{}(tools:{tool_size}):{}", pkg_name, pkg.annotation)
        })
        .collect();
    let mut lines = vec![];
    lines.push(format!(
        "# 基础信息:\n- 编号:{}\n- 名称:{}",
        identity.code(),
        identity.name(),
    ));
    lines.push(EVENT_DRIVEN.trim().to_string());
    lines.push(GENERATE_RULE.trim().to_string());
    lines.push(STACK_CONTEXT_RULE.trim().to_string());
    lines.push(TOOL_CALL_RULE.trim().to_string());
    lines.push(RENDER_RULE.trim().to_string());
    lines.push(format!("# 工具包列表(全部)\n{}", items.join("\n"),));
    lines.join("\n\n")
}
