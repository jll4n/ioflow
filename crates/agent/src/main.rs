use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod bridge_client;
mod poller;
mod runner;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "agent=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let backend_url =
        std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://localhost:3000".into());

    tracing::info!("agent starting, backend = {}", backend_url);

    let client = reqwest::Client::new();

    loop {
        match poller::poll(&client, &backend_url).await {
            Ok(Some(job)) => {
                tracing::info!(job_id = %job.id, "picked up job");
                runner::run(&client, &backend_url, job).await;
            }
            Ok(None) => {
                tracing::debug!("no job available, waiting...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                tracing::error!("poll error: {e}");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}
