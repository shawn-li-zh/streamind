use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, LazyLock},
};

use entity::event::EnvEventType;
use entity_service::event::event_convert;
use kernel::{
    agent::{context::AgentContext, AgentIdentity},
    client::ChatClient,
    mind::context::EventContext as _,
    tool::engine::ToolEngineContext,
};
use runtime::agent::{
    event::AppEnvEvent,
    event_context::{MindNode, Node, StackContext},
};
use tracing::info;

pub async fn load_stack(
    identity: &AgentIdentity,
    client: Arc<dyn ChatClient>,
    ctx: &AgentContext,
) -> anyhow::Result<StackContext<AppEnvEvent>> {
    let agent_id = identity.id();
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
        let node = stack.push_node(Node {
            id: state.id,
            pos: state.pos as _,
            title: state.title,
            content: state.content,
        });
        let events = state_id_map_events.remove(&state.id).unwrap_or_default();
        for event in events {
            let agent_event = event_convert::from_entity(event);
            node.push(agent_event);
        }
        info!("状态节点:{}，共加载{}个事件", node.pos, node.len());
    }
    //
    Ok(stack)
}

pub static EVENT_DRIVEN: LazyLock<String> =
    LazyLock::new(|| std::fs::read_to_string("prompts/common/event_driven.md").unwrap_or_default());

pub static GENERATE_RULE: LazyLock<String> = LazyLock::new(|| {
    std::fs::read_to_string("prompts/common/generate_rule.md").unwrap_or_default()
});
pub static STACK_CONTEXT_RULE: LazyLock<String> = LazyLock::new(|| {
    std::fs::read_to_string("prompts/common/stack_context_rule.md").unwrap_or_default()
});
pub static REFLECT_RULE: LazyLock<String> =
    LazyLock::new(|| std::fs::read_to_string("prompts/common/reflect_rule.md").unwrap_or_default());
pub static TOOL_CALL_RULE: LazyLock<String> = LazyLock::new(|| {
    std::fs::read_to_string("prompts/common/tool_call_rule.md").unwrap_or_default()
});
pub static RENDER_RULE: LazyLock<String> =
    LazyLock::new(|| std::fs::read_to_string("prompts/common/render_rule.md").unwrap_or_default());

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
    let dir = std::env::current_dir().unwrap();

    let mut lines = vec![];
    let workspace = identity.workspace.to_str().unwrap_or_default();
    lines.push(format!(
        "# 基础信息:\n- 编号:{}\n- 名称:{}\n- 系统工作目录(绝对路径,公共文档一般是以这个路径为基准):{}\n-工作空间是:{}",
        identity.code(),
        identity.name(),
        dir.to_str().unwrap_or_default(),
        workspace
    ));
    lines.push(EVENT_DRIVEN.trim().to_string());
    lines.push(GENERATE_RULE.trim().to_string());
    lines.push(STACK_CONTEXT_RULE.trim().to_string());
    lines.push(REFLECT_RULE.trim().to_string());
    lines.push(TOOL_CALL_RULE.trim().to_string());
    lines.push(RENDER_RULE.trim().to_string());
    lines.push(format!("# 工具包列表(全部)\n{}", items.join("\n"),));

    lines.join("\n\n")
}
