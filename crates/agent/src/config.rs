use uuid::Uuid;

pub struct Config {
    pub agent_id: Uuid,
    pub org_id: Uuid,
    pub backend_url: String,
    pub version: &'static str,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            agent_id: std::env::var("AGENT_ID")
                .expect("AGENT_ID must be set (UUID stable identifiant cet agent)")
                .parse()
                .expect("AGENT_ID doit être un UUID valide"),
            org_id: std::env::var("ORG_ID")
                .expect("ORG_ID must be set (UUID de l'organisation)")
                .parse()
                .expect("ORG_ID doit être un UUID valide"),
            backend_url: std::env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    pub fn hostname() -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }
}
