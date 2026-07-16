use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use shared::protocol::AgentRegisterRequest;

use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub fn router() -> Router<AppState> {
    Router::new().route("/register", post(register_agent))
}

async fn register_agent(
    State(state): State<AppState>,
    Json(body): Json<AgentRegisterRequest>,
) -> ApiResult<serde_json::Value> {
    sqlx::query(
        "INSERT INTO agents (id, org_id, hostname, version, last_seen_at)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (id) DO UPDATE
             SET hostname = EXCLUDED.hostname,
                 version  = EXCLUDED.version,
                 last_seen_at = NOW()",
    )
    .bind(body.agent_id)
    .bind(body.org_id)
    .bind(&body.hostname)
    .bind(&body.version)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("erreur DB register_agent : {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "erreur base de données" })),
        )
    })?;

    tracing::info!(
        agent_id = %body.agent_id,
        org_id   = %body.org_id,
        hostname = %body.hostname,
        "agent enregistré"
    );
    Ok(Json(serde_json::json!({ "ok": true })))
}
