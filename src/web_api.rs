use std::{convert::Infallible, sync::Arc};

use axum::{
    extract::Path,
    response::{sse::Event, Sse},
    routing::{get, post},
    Json, Router,
};

use futures::Stream;
use kernel::event::{EnvEvent, InteractionData};
use runtime::{agent::instance::AgentInstance, global::AGENT_POOL, sse_event::WebEvent};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tracing::info;
use web::wrapper::ApiResponse;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserMessageReq {
    pub content: String,
    pub attachments: Vec<String>,
}
pub async fn connect(
    Path(agent_id): Path<u32>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let p = AGENT_POOL.read().await;
    let agent_arc = match p.get(&agent_id) {
        Some(v) => v.clone(),
        None => {
            let agent = entity_service::agent::get_by_id(agent_id)
                .await
                .unwrap()
                .expect("agent不存在！");
            drop(p);
            let mut p = AGENT_POOL.write().await;
            let (agent, runime, mind) = AgentInstance::build(agent).await;
            runime.start(mind);
            agent.scheduler.clone().start().unwrap();
            // let agent = Arc::new(agent);
            p.insert(agent_id, agent);
            p.get(&agent_id).unwrap().clone()
        }
    };
    info!("connect:{agent_id}");
    // 2. 创建通道 (Unbounded 适合流式输出)
    let (tx, rx) = mpsc::unbounded_channel();

    // 3. 将写入器注入 Agent
    {
        let e= WebEvent::Connected;
        tx.send(e.to_sse_event()).unwrap();
        agent_arc.channel.set_web_sender(tx).await;
    }

    // 4. 将接收端转为 Stream 返回给前端
    let stream = UnboundedReceiverStream::new(rx).map(Ok);

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
#[derive(Debug, Deserialize)]
pub struct InteractionEvent {
    pub section_id: i64,
    pub message: String,
}
pub async fn ui_interaction(
    Path(agent_id): Path<u32>,
    Json(req): Json<InteractionEvent>,
) -> ApiResponse<entity::event::Model> {
    let event = EnvEvent::Interaction(InteractionData {
        section_id: req.section_id,
        message: req.message,
    });
    let p = AGENT_POOL.read().await;
    let agent_arc = p.get(&agent_id).expect("agent不存在！");
    let (_, model) = agent_arc.channel.send_event(event, false).await.unwrap();

    ApiResponse::success(model)
}

pub async fn user_interrupt(Path(agent_id): Path<u32>) -> ApiResponse<entity::event::Model> {
    let event = EnvEvent::Interrupt("用户发起中断指令".to_string());
    let p = AGENT_POOL.read().await;
    let agent_arc = p.get(&agent_id).expect("agent不存在！");

    let (_, model) = agent_arc.channel.send_event(event, false).await.unwrap();
    ApiResponse::success(model)
}

pub async fn chat(
    Path(agent_id): Path<u32>,
    Json(msg): Json<UserMessageReq>,
) -> ApiResponse<entity::event::Model> {
    let p = AGENT_POOL.read().await;
    let agent_arc = p.get(&agent_id).expect("agent不存在！");

    let (_, model) = agent_arc
        .channel
        .send_event(
            EnvEvent::UserRequest(kernel::event::UserMessage {
                content: msg.content,
                attachments: vec![],
            }),
            false,
        )
        .await
        .unwrap();

    ApiResponse::success(model)
}

pub fn inject_router(mut router: Router) -> Router {
    //agent
    router = router
        .route("/api/agent/connect/{agent_id}", get(connect))
        .route("/api/agent/chat/{agent_id}", post(chat))
        .route("/api/agent/user_interrupt/{agent_id}", post(user_interrupt))
        .route("/api/agent/ui_interaction/{agent_id}", post(ui_interaction));
    router
}
