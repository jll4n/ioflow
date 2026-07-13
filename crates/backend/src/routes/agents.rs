use axum::{routing::post, Json, Router};
use shared::protocol::AgentRegisterRequest;

pub fn router() -> Router {
    Router::new().route("/register", post(register_agent))
}

async fn register_agent(Json(body): Json<AgentRegisterRequest>) -> Json<serde_json::Value> {
    tracing::info!(agent_id = %body.agent_id, hostname = %body.hostname, "agent registered");
    // TODO: persist to DB
    Json(serde_json::json!({ "ok": true }))
}
