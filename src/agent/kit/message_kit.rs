use std::collections::HashMap;

use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs,
};
use entity::event::EventTypeName;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use streamind_core::agent::AgentContext;

use crate::{
    agent::{event::AgentEvent, kit::prompt_kit},
    global::DB,
};

pub fn build_step_messages(events: &Vec<AgentEvent>) -> ChatCompletionRequestMessage {
    //
    let mut msgs = vec![];
    //事件转消息
    for ele in events {
        let msg = ele.to_message();
        msgs.push(msg);
    }
    let c = msgs.join("\n\n");
    let msg = ChatCompletionRequestSystemMessageArgs::default()
        .content(c)
        .build()
        .unwrap();
    ChatCompletionRequestMessage::System(msg)
}

pub async fn load_messages(
    agent_id: u32,
    state_version: i64,
) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
    let db = DB.get().unwrap();

    let events = entity::event::Entity::find()
        .filter(entity::event::Column::AgentId.eq(agent_id))
        .filter(entity::event::Column::StateVersion.eq(state_version))
        .order_by_asc(entity::event::Column::Step)
        .all(db)
        .await?;
    let mut messages: Vec<ChatCompletionRequestMessage> = vec![];
    let mut steps = vec![];
    let mut last_step = 0;
    for event in events {
        if let EventTypeName::AgentResponse = &event.event_type {
            let sections = entity::section::Entity::find()
                .filter(entity::section::Column::AgentId.eq(agent_id))
                .filter(entity::section::Column::Step.eq(event.step))
                .all(db)
                .await?;
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
            messages.push(ChatCompletionRequestMessage::Assistant(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(response)
                    .build()
                    .unwrap(),
            ));
            continue;
        }
        if last_step != event.step {
            if !steps.is_empty() {
                let msg = build_step_messages(&steps);
                messages.push(msg);
            }
            last_step = event.step;
            steps = vec![];
            continue;
        }
        let event = AgentEvent::from_entity(event);
        steps.push(event);
    }

    Ok(messages)
}
//构造当前状态下的上下文
pub async fn build_state_context(ctx: &AgentContext) -> Vec<ChatCompletionRequestMessage> {
    // 加载系统提示词和历史消息
    let system_prompt = prompt_kit::build_system_prompt(&ctx.info).await;
    let mut messages = vec![prompt_kit::system_message(system_prompt)];
    let msgs = load_messages(ctx.info.id, ctx.info.state_version)
        .await
        .unwrap();
    messages.extend(msgs);
    messages
}
