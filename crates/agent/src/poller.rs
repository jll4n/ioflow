use shared::{job::Job, protocol::PollResponse};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn poll(client: &reqwest::Client, backend_url: &str) -> Result<Option<Job>, Error> {
    let resp: PollResponse = client
        .get(format!("{}/api/v1/jobs/poll", backend_url))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(resp.job)
}
