use shared::{
    job::{Job, JobStatus},
    protocol::UpdateJobRequest,
};

pub async fn run(client: &reqwest::Client, backend_url: &str, job: Job) {
    update_status(client, backend_url, &job, JobStatus::Running, None).await;

    match crate::bridge_client::execute_build(&job).await {
        Ok(result) => {
            let status = if result.success {
                JobStatus::Success
            } else {
                JobStatus::Failed
            };
            // Convert bridge result → shared JobResult
            let job_result = shared::job::JobResult {
                job_id: job.id,
                success: result.success,
                diagnostics: result.diagnostics,
                duration_ms: result.duration_ms,
                agent_id: uuid::Uuid::nil(), // TODO: real agent ID from config
            };
            update_status(client, backend_url, &job, status, Some(job_result)).await;
        }
        Err(e) => {
            tracing::error!(job_id = %job.id, "bridge error: {e}");
            update_status(client, backend_url, &job, JobStatus::Failed, None).await;
        }
    }
}

async fn update_status(
    client: &reqwest::Client,
    backend_url: &str,
    job: &Job,
    status: JobStatus,
    result: Option<shared::job::JobResult>,
) {
    let url = format!("{}/api/v1/jobs/{}/status", backend_url, job.id);
    let body = UpdateJobRequest { status, result };
    if let Err(e) = client.post(&url).json(&body).send().await {
        tracing::error!(job_id = %job.id, "failed to update status: {e}");
    }
}
