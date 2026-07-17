use axum::{response::Html, routing::get, Router};
use sqlx::PgPool;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod routes;

use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();

    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("connexion PostgreSQL échouée");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations échouées");

    let state = AppState { db: pool };

    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../../../web/templates/dashboard.html")) }))
        .route("/health", get(|| async { "ok" }))
        .nest("/api/v1", routes::router())
        .with_state(state);

    let addr: SocketAddr = config.bind_addr.parse().expect("BIND_ADDR invalide");
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
