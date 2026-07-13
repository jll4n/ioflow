use axum::{extract::Path, routing::{get, post}, Json, Router};
use shared::protocol::{PollResponse, UpdateJobRequest};
use uuid::Uuid;

pub fn router() -> Router {
    Router::new()
        .route("/poll", get(poll_job))
        .route("/:id/status", post(update_job_status))
}

async fn poll_job() -> Json<PollResponse> {
    // TODO: query DB for next queued job
    Json(PollResponse { job: None })
}

async fn update_job_status(
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateJobRequest>,
) -> Json<serde_json::Value> {
    tracing::info!(job_id = %id, status = ?body.status, "job status update");
    // TODO: persist to DB
    Json(serde_json::json!({ "ok": true }))
}
