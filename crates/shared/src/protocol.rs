/// Protocole HTTP entre l'agent et le backend (corps JSON des requêtes/réponses).
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::job::{Job, JobResult, JobStatus};

#[derive(Debug, Serialize, Deserialize)]
pub struct PollResponse {
    pub job: Option<Job>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateJobRequest {
    pub status: JobStatus,
    pub result: Option<JobResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRegisterRequest {
    pub agent_id: Uuid,
    pub hostname: String,
    pub version: String,
}
