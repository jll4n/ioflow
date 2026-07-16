use axum::{routing::post, Json, Router};
use shared::protocol::AgentRegisterRequest;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/register", post(register_agent))
}

// L'insertion en base nécessite org_id, qui sera résolu par le contexte de session
// une fois l'auth implémentée. En attendant, on log et on répond 200.
async fn register_agent(Json(body): Json<AgentRegisterRequest>) -> Json<serde_json::Value> {
    tracing::info!(
        agent_id = %body.agent_id,
        hostname = %body.hostname,
        version  = %body.version,
        "agent enregistré (persistance en attente de l'auth)"
    );
    Json(serde_json::json!({ "ok": true }))
}
