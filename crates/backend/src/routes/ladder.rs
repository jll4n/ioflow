use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Form, Json, Router,
};
use plcopen::{
    renderer::{diff::render_diff, svg::render_network},
    Body, LdNetwork, Project,
};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;

use crate::AppState;

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        // Endpoints sans état (dashboard htmx)
        .route("/render/ladder", post(render_ladder_form))
        .route("/render/ladder-diff", post(render_ladder_diff_form))
        // Endpoints basés sur les snapshots en base
        .route("/snapshots", post(create_snapshot))
        .route("/snapshots/:hash/pous", get(list_pous))
        .route("/snapshots/:hash/pou/:name/ladder", get(get_ladder))
        .route("/diff/:h1/:h2/pou/:name/ladder", get(get_diff))
}

// ─── Formulaires htmx (sans base de données) ─────────────────────────────────

#[derive(Deserialize)]
pub struct RenderForm {
    xml: String,
    pou: String,
    #[serde(default)]
    network: usize,
}

#[derive(Deserialize)]
pub struct DiffForm {
    xml_a: String,
    xml_b: String,
    pou: String,
    #[serde(default)]
    network: usize,
}

async fn render_ladder_form(Form(body): Form<RenderForm>) -> Response {
    match do_render(&body.xml, &body.pou, body.network) {
        Ok(svg) => svg_ok(svg),
        Err(msg) => svg_error(&msg),
    }
}

async fn render_ladder_diff_form(Form(body): Form<DiffForm>) -> Response {
    let proj_a = match plcopen::parse_project(&body.xml_a) {
        Ok(p) => p,
        Err(e) => return svg_error(&format!("XML A invalide : {e}")),
    };
    let proj_b = match plcopen::parse_project(&body.xml_b) {
        Ok(p) => p,
        Err(e) => return svg_error(&format!("XML B invalide : {e}")),
    };

    match (
        find_ld_network(&proj_a, &body.pou, body.network),
        find_ld_network(&proj_b, &body.pou, body.network),
    ) {
        (Ok(na), Ok(nb)) => svg_ok(render_diff(na, nb)),
        (Err(e), _) | (_, Err(e)) => svg_error(&e),
    }
}

// ─── Endpoints snapshots (base de données) ───────────────────────────────────

#[derive(Deserialize)]
pub struct SnapshotPayload {
    pub commit_hash: String,
    pub xml_content: String,
}

#[derive(Serialize)]
pub struct PouInfo {
    pub name: String,
    pub lang: String,
}

#[derive(Deserialize)]
pub struct NetworkQuery {
    #[serde(default)]
    pub network: usize,
}

async fn create_snapshot(
    State(state): State<AppState>,
    Json(body): Json<SnapshotPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Valider le XML avant stockage
    plcopen::parse_project(&body.xml_content).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("XML invalide : {e}") })),
        )
    })?;

    sqlx::query(
        "INSERT INTO plcopen_snapshots (commit_hash, xml_content) VALUES ($1, $2) \
         ON CONFLICT (commit_hash) DO UPDATE SET xml_content = EXCLUDED.xml_content",
    )
    .bind(&body.commit_hash)
    .bind(&body.xml_content)
    .execute(&state.db)
    .await
    .map_err(db_err)?;

    tracing::info!(hash = %body.commit_hash, "snapshot PLCopen stocké");
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_pous(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<Vec<PouInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let xml = fetch_xml(&state, &hash).await?;
    let project = parse_or_err(&xml)?;

    let pous = project
        .pous
        .iter()
        .map(|p| PouInfo {
            name: p.name.clone(),
            lang: match &p.body {
                Body::Ld(_) => "LD",
                Body::Fbd(_) => "FBD",
                Body::St(_) => "ST",
                Body::Il(_) => "IL",
                Body::Sfc(_) => "SFC",
            }
            .to_string(),
        })
        .collect();

    Ok(Json(pous))
}

async fn get_ladder(
    State(state): State<AppState>,
    Path((hash, pou_name)): Path<(String, String)>,
    Query(q): Query<NetworkQuery>,
) -> Response {
    let xml = match fetch_xml(&state, &hash).await {
        Ok(x) => x,
        Err((code, msg)) => {
            return (code, msg).into_response();
        }
    };
    match do_render(&xml, &pou_name, q.network) {
        Ok(svg) => svg_ok(svg),
        Err(msg) => svg_error(&msg),
    }
}

async fn get_diff(
    State(state): State<AppState>,
    Path((h1, h2, pou_name)): Path<(String, String, String)>,
    Query(q): Query<NetworkQuery>,
) -> Response {
    let xml_a = match fetch_xml(&state, &h1).await {
        Ok(x) => x,
        Err((c, m)) => return (c, m).into_response(),
    };
    let xml_b = match fetch_xml(&state, &h2).await {
        Ok(x) => x,
        Err((c, m)) => return (c, m).into_response(),
    };

    let proj_a = match parse_or_err(&xml_a) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let proj_b = match parse_or_err(&xml_b) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };

    match (
        find_ld_network(&proj_a, &pou_name, q.network),
        find_ld_network(&proj_b, &pou_name, q.network),
    ) {
        (Ok(na), Ok(nb)) => svg_ok(render_diff(na, nb)),
        (Err(e), _) | (_, Err(e)) => svg_error(&e),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn do_render(xml: &str, pou_name: &str, net_idx: usize) -> Result<String, String> {
    let project = plcopen::parse_project(xml).map_err(|e| e.to_string())?;
    let net = find_ld_network(&project, pou_name, net_idx)?;
    Ok(render_network(net))
}

fn find_ld_network<'a>(
    project: &'a Project,
    pou_name: &str,
    net_idx: usize,
) -> Result<&'a LdNetwork, String> {
    let pou = project
        .pous
        .iter()
        .find(|p| p.name == pou_name)
        .ok_or_else(|| format!("POU '{pou_name}' introuvable"))?;

    match &pou.body {
        Body::Ld(ld) => ld
            .networks
            .get(net_idx)
            .ok_or_else(|| format!("réseau {net_idx} introuvable dans '{pou_name}'")),
        _ => Err(format!("POU '{pou_name}' n'est pas en Ladder")),
    }
}

async fn fetch_xml(
    state: &AppState,
    hash: &str,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query(
        "SELECT xml_content FROM plcopen_snapshots WHERE commit_hash = $1",
    )
    .bind(hash)
    .fetch_optional(&state.db)
    .await
    .map_err(db_err)?;

    row.map(|r| r.get::<String, _>("xml_content")).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("snapshot '{hash}' introuvable") })),
        )
    })
}

fn parse_or_err(
    xml: &str,
) -> Result<Project, (StatusCode, Json<serde_json::Value>)> {
    plcopen::parse_project(xml).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("XML invalide : {e}") })),
        )
    })
}

fn svg_ok(svg: String) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    )
        .into_response()
}

fn svg_error(msg: &str) -> Response {
    let escaped = msg.replace('&', "&amp;").replace('<', "&lt;");
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"500\" height=\"40\">\
<text x=\"8\" y=\"24\" fill=\"#dc2626\" font-family=\"monospace\" font-size=\"12\">Erreur : {escaped}</text>\
</svg>"
    );
    (
        StatusCode::BAD_REQUEST,
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    )
        .into_response()
}

fn db_err(e: sqlx::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!("erreur DB ladder : {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "erreur base de données" })),
    )
}
