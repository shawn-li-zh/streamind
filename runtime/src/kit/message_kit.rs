use std::{collections::HashMap, fs::File};

use async_openai::types::chat::ChatCompletionRequestMessage;
use entity::event::EnvEventType;

use kernel::{agent::AgentContext, event::EnvEvent, message::Message, section::Section};

use crate::{
    context::stack::{MindNode, MindStack},
    kit::prompt_kit,
};

pub fn to_sdk_message(value: Message) -> ChatCompletionRequestMessage {
    let role = value.get_role();
    let (text, paths) = value.to_tuple();
    match role {
        kernel::message::Role::User => prompt_kit::user_message(text, paths),
        kernel::message::Role::System => prompt_kit::system_message(text),
        kernel::message::Role::Tool => prompt_kit::tool_message(text, paths),
        kernel::message::Role::Assistant => prompt_kit::assistant_message(text),
    }
}

pub fn to_assistant_ouput(sections: Vec<Section>) -> Option<String> {
    if sections.is_empty() {
        return None;
    }
    let has_tool_call = sections.iter().any(|s| s.name == "tool_call");
    fn join(sections: Vec<Section>) -> String {
        let mut text = String::new();
        for section in sections {
            text += &section.to_string();
            text += "\n";
        }
        text
    }
    if has_tool_call {
        Some(join(sections))
    } else {
        //去除思考内容
        let sections: Vec<Section> = sections.into_iter().filter(|s| s.name != "think").collect();
        if sections.is_empty() {
            None
        } else {
            Some(join(sections))
        }
    }
}

pub async fn load_stack(agent_ctx: &AgentContext) -> anyhow::Result<MindStack> {
    let (agent_id, agent_code, agent_name, workspace) = {
        let info = agent_ctx.info.read().await;
        (
            info.id,
            info.code.clone(),
            info.name.clone(),
            info.workspace.clone(),
        )
    };
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
    let sp = prompt_kit::build_system_prompt(agent_ctx, &agent_code, &agent_name);
    let path = workspace.join("log.txt");
    let file = File::options().append(true).create(true).open(path)?;
    let mut stack = MindStack::new(sp, file);
    for state in states {
        let node = MindNode {
            id: state.id,
            pos: state.pos as _,
            title: state.title,
            content: state.content,
            messages: vec![],
            content_len: 0,
        };
        stack.push_node(node);
        //
        let events = state_id_map_events.remove(&state.id).unwrap_or_default();
        for event in events {
            let agent_event = entity_service::event::convert::from_entity(event);
      
            if let EnvEvent::AgentResponse(s) = &agent_event.data {
                if let EnvEvent::AgentResponse(sections) = agent_event.data {
                    stack.push_response(sections);
                    continue;
                }
            }
            let msg = agent_event.to_message();
            stack.push_msg(msg);
        }
        //
    }
    //
    Ok(stack)
}
