use shared::bridge::{BridgeCommand, BridgeResponse, BuildResult};

#[cfg(feature = "com")]
mod ude;

pub fn handle(cmd: BridgeCommand) -> BridgeResponse {
    match cmd {
        BridgeCommand::Ping => BridgeResponse::Pong,
        BridgeCommand::OpenProject { path } => open_project(&path),
        BridgeCommand::Build { job_id } => build(job_id),
        BridgeCommand::CloseProject => close_project(),
    }
}

fn open_project(path: &str) -> BridgeResponse {
    #[cfg(feature = "com")]
    return ude::open_project(path);

    #[cfg(not(feature = "com"))]
    {
        eprintln!("[mock] open_project: {path}");
        BridgeResponse::ProjectOpened
    }
}

fn build(job_id: uuid::Uuid) -> BridgeResponse {
    #[cfg(feature = "com")]
    return ude::build(job_id);

    #[cfg(not(feature = "com"))]
    {
        eprintln!("[mock] build: job_id={job_id}");
        BridgeResponse::BuildResult(BuildResult {
            job_id,
            success: true,
            diagnostics: vec![],
            duration_ms: 0,
        })
    }
}

fn close_project() -> BridgeResponse {
    #[cfg(feature = "com")]
    return ude::close_project();

    #[cfg(not(feature = "com"))]
    {
        eprintln!("[mock] close_project");
        BridgeResponse::ProjectClosed
    }
}
