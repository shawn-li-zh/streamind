use axum::{
    Router,
    routing::{delete, get, post, put},
};

mod agent;
mod event;
mod model;
mod render_component;
mod render_dependence;
// mod render_state;
mod section;
mod state;

pub fn inject_router(mut router: Router) -> Router {
    //模型
    router = router
        .route("/api/model", post(model::create))
        .route("/api/model/{id}", put(model::update))
        .route("/api/model/list", get(model::list));
    //agent
    router = router
        .route("/api/agent", post(agent::create))
        .route("/api/agent/{id}", get(agent::get_by_id))
        .route("/api/agent/tree", get(agent::tree))
        // .route("/api/agent/connect/{agent_id}", get(agent::connect))
        // .route("/api/agent/chat/{agent_id}", post(agent::chat))
        // .route(
        //     "/api/agent/ui_interaction/{agent_id}",
        //     post(agent::ui_interaction),
        // )
        ;
    //状态
    router = router.route("/api/state/list", get(state::list));
    //事件
    router = router
        .route("/api/event/list", get(event::list))
        .route("/api/event/count_by_status/{id}", get(event::count_by_status))
        .route("/api/event/load", get(event::load));
    //响应
    router = router
        .route("/api/section/list", get(section::list))
        .route("/api/section/list_in_range", get(section::list_in_range))
        .route("/api/section/{id}", get(section::get_by_id));
    //组件
    router = router.route(
        "/api/render_component/{id}",
        get(render_component::get_by_id),
    );
    //依赖
    router = router
        .route("/api/render_dependence/list", get(render_dependence::list))
        .route("/api/render_dependence", post(render_dependence::create))
        .route("/api/render_dependence", put(render_dependence::update))
        .route(
            "/api/render_dependence/{id}",
            delete(render_dependence::remove),
        )
        .route(
            "/api/render_dependence/download_to_local/{id}",
            get(render_dependence::download_to_local),
        );
    //渲染
    // router = router
    //     .route("/api/render_state/{id}", get(render_state::get_by_id))
    //     .route(
    //         "/api/render_state/get_by_pos",
    //         get(render_state::get_by_pos),
    //     )
    //     .route(
    //         "/api/render_state/state/{id}",
    //         post(render_state::set_state_by_id),
    //     );
    router
}
