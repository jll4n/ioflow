use axum::Router;

mod agents;
mod jobs;

pub fn router() -> Router {
    Router::new()
        .nest("/jobs", jobs::router())
        .nest("/agents", agents::router())
}
