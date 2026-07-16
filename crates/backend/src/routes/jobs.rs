use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use shared::{
    job::{DiagnosticLevel, Job, JobKind, JobStatus},
    protocol::{PollResponse, UpdateJobRequest},
};
use sqlx::Row as _;
use uuid::Uuid;

use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/poll", get(poll_job))
        .route("/:id/status", post(update_job_status))
}

async fn poll_job(State(state): State<AppState>) -> ApiResult<PollResponse> {
    let mut tx = state.db.begin().await.map_err(db_err)?;

    let row = sqlx::query(
        "SELECT id, project_id, created_at FROM jobs \
         WHERE status = 'queued' ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?;

    let row = match row {
        None => {
            tx.commit().await.map_err(db_err)?;
            return Ok(Json(PollResponse { job: None }));
        }
        Some(r) => r,
    };

    let job_id: Uuid = row.get("id");
    let project_id: Uuid = row.get("project_id");
    let created_at: DateTime<Utc> = row.get("created_at");

    sqlx::query("UPDATE jobs SET status = 'running', started_at = NOW() WHERE id = $1")
        .bind(job_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

    tx.commit().await.map_err(db_err)?;

    tracing::info!(job_id = %job_id, "job assigné à un agent");

    Ok(Json(PollResponse {
        job: Some(Job {
            id: job_id,
            project_id,
            kind: JobKind::Build,
            status: JobStatus::Running,
            created_at,
        }),
    }))
}

async fn update_job_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateJobRequest>,
) -> ApiResult<serde_json::Value> {
    let status_str = match body.status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Success => "success",
        JobStatus::Failed => "failed",
    };

    sqlx::query("UPDATE jobs SET status = $1, finished_at = NOW() WHERE id = $2")
        .bind(status_str)
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(db_err)?;

    if let Some(result) = &body.result {
        for diag in &result.diagnostics {
            let level_str = match diag.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info => "info",
            };
            sqlx::query(
                "INSERT INTO diagnostics (job_id, level, message, location) VALUES ($1, $2, $3, $4)",
            )
            .bind(id)
            .bind(level_str)
            .bind(&diag.message)
            .bind(&diag.location)
            .execute(&state.db)
            .await
            .map_err(db_err)?;
        }
    }

    tracing::info!(job_id = %id, status = status_str, "job status mis à jour");
    Ok(Json(serde_json::json!({ "ok": true })))
}

fn db_err(e: sqlx::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!("erreur DB : {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "erreur base de données" })),
    )
}
