use std::{collections::HashMap, fs::File};

use async_openai::types::ChatCompletionRequestMessage;
use entity::event::EnvEventType;

use kernel::{
    agent::{AgentContext, AgentEvent, AgentInfo},
    event::EnvEvent,
    message::Message,
};

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
        kernel::message::Role::ToolCall => prompt_kit::tool_message(text,paths),
        kernel::message::Role::Assistant => prompt_kit::assistant_message(text),
    }
}

pub async fn load_stack(agent_ctx: &AgentContext) -> anyhow::Result<MindStack> {
    let agent_id = agent_ctx.info.id;
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
    let sp = prompt_kit::build_system_prompt(agent_ctx);
    let path = agent_ctx.info.workspace.join("log.txt");
    let file = File::options().append(true).create(true).open(path)?;
    let mut stack = MindStack::new(sp, file);
    for state in states {
        let mut node = MindNode {
            id: state.id,
            pos: state.pos as _,
            title: state.title,
            content: state.content,
            messages: vec![],
            content_len: 0,
        };
        //
        let events = state_id_map_events.remove(&state.id).unwrap_or_default();
        for event in events {
            // if let EnvEventType::AgentResponse = &event.event_type {
            //     let sections = event_id_map_sections.remove(&event.id).unwrap_or_default();
            //     let response = sections
            //         .into_iter()
            //         .filter(|v| !v.payload.is_null())
            //         .map(|v| {
            //             let map: HashMap<String, serde_json::Value> =
            //                 serde_json::from_value(v.payload).unwrap();
            //             let content = map
            //                 .get("content")
            //                 .map(|v| v.as_str().unwrap_or_default())
            //                 .unwrap_or_default();
            //             format!("<{}>\n{}\n</{}>", v.name, content, v.name)
            //         })
            //         .collect::<Vec<String>>()
            //         .join("\n");
            //     if !response.is_empty() {
            //         let len = response.len();
            //         node.push(prompt_kit::assistant_message(response), len);
            //     }
            //     continue;
            // }
            let agent_event = entity_service::event::convert::from_entity(event);
            if let EnvEvent::AgentReaction(s) = &agent_event.data {
                if s.is_empty() {
                    continue;
                }
            }
            if let EnvEvent::AgentResponse(s) = &agent_event.data {
                if s.is_empty() {
                    continue;
                }
            }
            let msg = agent_event.to_message();
            let len = msg.size;
            node.push(to_sdk_message(msg), len);
        }
        //
        stack.push(stack.nodes.len(), node);
    }
    //
    Ok(stack)
}
