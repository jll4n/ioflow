use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod bridge_client;
mod config;
mod poller;
mod register;
mod runner;

use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "agent=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env();
    let hostname = Config::hostname();
    let client = reqwest::Client::new();

    tracing::info!(
        agent_id = %cfg.agent_id,
        org_id   = %cfg.org_id,
        backend  = %cfg.backend_url,
        "agent démarré"
    );

    if let Err(e) = register::register(
        &client,
        &cfg.backend_url,
        cfg.agent_id,
        cfg.org_id,
        hostname,
        cfg.version,
    )
    .await
    {
        tracing::warn!("enregistrement backend échoué (retry au prochain démarrage) : {e}");
    } else {
        tracing::info!("agent enregistré");
    }

    loop {
        match poller::poll(&client, &cfg.backend_url).await {
            Ok(Some(job)) => {
                tracing::info!(job_id = %job.id, "job récupéré");
                runner::run(&client, &cfg.backend_url, job, cfg.agent_id).await;
            }
            Ok(None) => {
                tracing::debug!("aucun job disponible, attente...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                tracing::error!("erreur de polling : {e}");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}
