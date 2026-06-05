use std::convert::Infallible;

use axum::{
    extract::Path,
    response::{sse, Sse},
    routing::{get, post},
    Json, Router,
};

use futures::Stream;


use runtime::agent::event::{AppEnvEventData, InteractionData, UserMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tracing::info;
use web::wrapper::ApiResponse;

use crate::{
    agent::app::AppAgent,
    global::AGENT_POOL,
    sse_event::{ServerEvent, WebEvent},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserMessageReq {
    pub content: String,
    pub attachments: Vec<String>,
}
pub async fn connect(
    Path(agent_id): Path<u32>,
) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let agent_arc = {
        let p = AGENT_POOL.read();
        p.get(&agent_id).map(|v| v.clone())
    };

    let agent_arc = match agent_arc {
        Some(v) => v.clone(),
        None => {
            let agent = {
                let agent = entity_service::agent::get_by_id(agent_id)
                    .await
                    .unwrap()
                    .expect("agent不存在！");
                let (agent, runtime) = AppAgent::new(agent).await;
                runtime.start();
                agent
            };
            let mut p = AGENT_POOL.write();
            // let agent = Arc::new(agent);
            p.insert(agent_id, agent.clone());
            agent
        }
    };
    info!("connect:{agent_id}");
    // 2. 创建通道 (Unbounded 适合流式输出)
    let (tx, rx) = mpsc::unbounded_channel();

    // 3. 将写入器注入 Agent
    {
        let e = WebEvent::Server(ServerEvent::Connected);
        tx.send(e.to_sse_event()).unwrap();
        agent_arc.web_sender.set_sender(tx);
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
) -> ApiResponse<()> {
    let agent_arc = {
        let p = AGENT_POOL.read();
        p.get(&agent_id).map(|v| v.clone())
    };
    let event = AppEnvEventData::Interaction(InteractionData {
        section_id: req.section_id,
        message: req.message,
    });
    if let Some(agent_arc) = agent_arc {
        match agent_arc.send_event((event, false)).await {
            Ok(_) => {
                return ApiResponse::ok();
            }
            Err(e) => {
                return ApiResponse::error(e);
            }
        }
    } else {
        ApiResponse::error("agent不存在！")
    }
}

pub async fn user_interrupt(Path(agent_id): Path<u32>) -> ApiResponse<()> {
    let agent_arc = {
        let p = AGENT_POOL.read();
        p.get(&agent_id).map(|v| v.clone())
    };
    let event = AppEnvEventData::Interrupt("用户发起中断指令".to_string());

    if let Some(agent_arc) = agent_arc {
        match agent_arc.send_event((event, true)).await {
            Ok(_) => {
                return ApiResponse::ok();
            }
            Err(e) => {
                return ApiResponse::error(e);
            }
        }
    } else {
        ApiResponse::error("agent不存在！")
    }
}

pub async fn chat(Path(agent_id): Path<u32>, Json(msg): Json<UserMessageReq>) -> ApiResponse<()> {
    let agent_arc = {
        let p = AGENT_POOL.read();
        p.get(&agent_id).map(|v| v.clone())
    };

    if let Some(agent_arc) = agent_arc {
        let event: (AppEnvEventData, bool) = (
            AppEnvEventData::UserRequest(UserMessage {
                content: msg.content.trim().to_string(),
                attachments: vec![],
            }),
            true,
        );
        match agent_arc.send_event(event).await {
            Ok(_) => {
                return ApiResponse::ok();
            }
            Err(e) => {
                return ApiResponse::error(e);
            }
        }
    } else {
        ApiResponse::error("agent不存在！")
    }
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
