use axum::{
    routing::{get, post},
    Router,
};

mod agent;
mod event;
mod render_component;
mod render_state;
mod section;
mod state;

pub fn inject_router(mut router: Router) -> Router {
    //agent
    router = router
        .route("/api/agent/:id", get(agent::get_by_id))
        .route("/api/agent/connect/:agent_id", get(agent::connect))
        .route("/api/agent/tree", get(agent::tree))
        .route("/api/agent/chat/:agent_id", post(agent::chat))
        .route(
            "/api/agent/ui_interaction/:agent_id",
            post(agent::ui_interaction),
        );
    //状态
    router = router.route("/api/state/list", get(state::list));
    //事件
    router = router
        .route("/api/event/list", get(event::list))
        .route("/api/event/load", get(event::load));
    //响应
    router = router
        .route("/api/section/list", get(section::list))
        .route("/api/section/:id", get(section::get_by_id));
    //组件
    router = router.route(
        "/api/render_component/:id",
        get(render_component::get_by_id),
    );
    //渲染
    router = router
        .route("/api/render_state/:id", get(render_state::get_by_id))
        .route(
            "/api/render_state/get_by_pos",
            get(render_state::get_by_pos),
        )
        .route(
            "/api/render_state/state/:id",
            post(render_state::set_state_by_id),
        );
    router
}
