use std::collections::HashMap;

use async_openai::types::ChatCompletionRequestMessage;
use entity::event::EnvEventType;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::{
    agent::{
        context::stack::{MindNode, MindStack},
        event::AgentEvent,
        kit::prompt_kit,
    },
    global::DB,
};

pub fn build_step_messages(events: &Vec<AgentEvent>) -> (ChatCompletionRequestMessage, usize) {
    //
    let mut msgs = vec![];
    //事件转消息
    for ele in events {
        let msg = ele.to_message();
        msgs.push(msg);
    }
    let c = msgs.join("\n\n");
    let size = c.len();
    let msg = prompt_kit::system_message(c);
    (msg, size)
}

pub async fn load_stack(agent_id: u32) -> anyhow::Result<MindStack> {
    let db = DB.get().unwrap();
    //查状态栈
    let states = entity::state::Entity::find()
        .filter(entity::state::Column::AgentId.eq(agent_id))
        .filter(entity::state::Column::Dumped.eq(false))
        .order_by_asc(entity::state::Column::Pos)
        .all(db)
        .await?;
    //查事件
    let state_ids: Vec<i64> = states.iter().map(|s| s.id).collect();
    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::StateId.is_in(state_ids))
        .order_by_asc(entity::event::Column::Step)
        .all(db)
        .await?;
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
    let sections = entity::section::Entity::find()
        .filter(entity::section::Column::AgentId.eq(agent_id))
        .filter(entity::section::Column::EventId.is_in(response_event_ids))
        .all(db)
        .await?;
    //
    let mut event_id_map_sections: HashMap<i64, Vec<entity::section::Model>> = HashMap::new();
    for s in sections {
        let arr = event_id_map_sections.entry(s.event_id).or_default();
        arr.push(s);
    }
    //
    let mut stack = MindStack::new();
    for state in states {
        let mut node = MindNode {
            id: state.id,
            title: state.title,
            content: state.content,
            messages: vec![],
            content_len: 0,
        };
        //
        let events = state_id_map_events.remove(&state.id).unwrap_or_default();
        let mut steps = vec![];
        let mut last_step = 0;
        for event in events {
            if let EnvEventType::AgentResponse = &event.event_type {
                let sections = event_id_map_sections.remove(&event.id).unwrap_or_default();
                let response = sections
                    .into_iter()
                    .map(|v| {
                        let map: HashMap<String, serde_json::Value> =
                            serde_json::from_value(v.meta).unwrap();
                        let content = map
                            .get("content")
                            .map(|v| v.as_str().unwrap_or_default())
                            .unwrap_or_default();
                        format!("<{}>\n{}\n</{}>", v.name, content, v.name)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");
                let len = response.len();
                node.push(prompt_kit::assistant_message(response), len);
                continue;
            }
            if last_step != event.step {
                if !steps.is_empty() {
                    let (msg, len) = build_step_messages(&steps);
                    node.push(msg, len);
                }
                last_step = event.step;
                steps = vec![];
                continue;
            }
            let event = AgentEvent::from_entity(event);
            steps.push(event);
        }
        //
        stack.push(node);
    }
    //
    Ok(stack)
}
