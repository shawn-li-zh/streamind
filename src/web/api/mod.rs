use axum::{
    routing::{get, post},
    Router,
};

mod agent;
mod event;
mod section;
mod state;

pub fn inject_router(mut router: Router) -> Router {
    //agent
    router = router
        .route("/api/agent/connect/:agent_id", get(agent::connect))
        .route("/api/agent/chat/:agent_id", post(agent::chat))
        .route("/api/agent/tree", get(agent::tree));
    //状态
    router = router.route("/api/state/list", get(state::list));
    //事件
    router = router
        .route("/api/event/list", get(event::list))
        .route("/api/event/load", get(event::load));
    //响应
    router = router.route("/api/section/list", get(section::list));
    router
}
