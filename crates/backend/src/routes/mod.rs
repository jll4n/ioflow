use axum::Router;

use crate::AppState;

mod agents;
mod jobs;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/jobs", jobs::router())
        .nest("/agents", agents::router())
}
