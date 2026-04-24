use std::{collections::HashMap, convert::Infallible, sync::Arc};

use crate::{
    agent::{event::AgentEventData, global::AGENT_POOL},
    global::DB,
    web::ApiResponse,
};
use axum::{
    extract::{Path, State},
    response::{sse::Event, Sse},
    Json,
};
use futures::Stream;
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserMessage {
    pub content: String,
    pub attachments: Vec<String>,
}
pub async fn connect(
    Path(agent_id): Path<u32>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("请求===");
    let p = AGENT_POOL.read().await;
    let agent_arc = p.get(&agent_id).expect("agent不存在！");

    // 2. 创建通道 (Unbounded 适合流式输出)
    let (tx, rx) = mpsc::unbounded_channel();

    // 3. 将写入器注入 Agent
    {
        let mut agent = agent_arc.write().await;
        agent.channel.web_sender = Some(tx);
    }

    // 4. 将接收端转为 Stream 返回给前端
    let stream = UnboundedReceiverStream::new(rx).map(Ok);

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn chat(
    Path(agent_id): Path<u32>,
    Json(msg): Json<UserMessage>,
) -> ApiResponse<entity::event::Model> {
    let p = AGENT_POOL.read().await;
    let agent_arc = p.get(&agent_id).expect("agent不存在！");
    let mut agent = agent_arc.write().await;
    let model = agent
        .send_event(AgentEventData::UserInput(msg.content), false)
        .await
        .unwrap();

    ApiResponse::success(model)
}

pub async fn tree() -> ApiResponse<Vec<entity::agent::Model>> {
    let agents = entity::agent::Entity::find()
        .order_by_asc(entity::agent::Column::ParentId)
        .all(DB.get().unwrap())
        .await
        .unwrap();
    let mut id_map_agent = HashMap::new();
    for agent in agents {
        if agent.parent_id == 0 {
            id_map_agent.insert(agent.id, agent);
            continue;
        }
        if let Some(parent) = id_map_agent.get_mut(&agent.parent_id) {
            parent.children.push(agent);
        }
    }
    ApiResponse::success(id_map_agent.into_values().collect())
}
