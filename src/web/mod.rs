use std::{convert::Infallible, sync::Arc};

use axum::{
    extract::{Path, State},
    response::{sse::Event, Sse},
    routing::get,
    Router,
};
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

use crate::agent::global::AGENT_POOL;

async fn connect_agent(
    Path(agent_id): Path<u32>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let p = AGENT_POOL.read().await;

    let agent_arc = match p.get(&agent_id) {
        Some(a) => a,
        None => {
            panic!("agent不存在！")
        }
    };

    // 2. 创建通道 (Unbounded 适合流式输出)
    let (tx, rx) = mpsc::unbounded_channel();

    // 3. 将写入器注入 Agent
    {
        let mut agent = agent_arc.write().await;
        agent.web_sender = Some(tx);
    }

    // 4. 将接收端转为 Stream 返回给前端
    let stream = UnboundedReceiverStream::new(rx).map(Ok);

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub fn inject_router(router: Router) -> Router {
    router.route("/api/connect/:agent_id", get(connect_agent))
}
