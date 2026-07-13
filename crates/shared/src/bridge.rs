/// Protocole IPC entre l'agent (x64) et le com-bridge (x86).
/// Transport : JSON newline-delimited sur stdin/stdout du sous-process.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::job::Diagnostic;

/// Commande envoyée par l'agent vers le com-bridge (une ligne JSON par message).
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum BridgeCommand {
    Ping,
    OpenProject { path: String },
    Build { job_id: Uuid },
    CloseProject,
}

/// Réponse écrite par le com-bridge sur stdout (une ligne JSON par message).
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeResponse {
    Pong,
    ProjectOpened,
    BuildResult(BuildResult),
    ProjectClosed,
    Error { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResult {
    pub job_id: Uuid,
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub duration_ms: u64,
}
