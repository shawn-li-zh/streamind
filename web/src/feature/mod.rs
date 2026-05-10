use axum::{http::HeaderValue, routing::get, Router};
use reqwest::{header, Method};
use tower_http::cors::{Any, CorsLayer};

// pub mod render;
pub mod auth;
pub mod public;

pub fn inject_router(mut router: Router) -> Router {
    router = public::inject_router(router);
    router = auth::inject_router(router);
    router
}
