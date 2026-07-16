use shared::protocol::AgentRegisterRequest;
use uuid::Uuid;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn register(
    client: &reqwest::Client,
    backend_url: &str,
    agent_id: Uuid,
    org_id: Uuid,
    hostname: String,
    version: &str,
) -> Result<(), Error> {
    let body = AgentRegisterRequest {
        agent_id,
        org_id,
        hostname,
        version: version.to_string(),
    };
    client
        .post(format!("{}/api/v1/agents/register", backend_url))
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
