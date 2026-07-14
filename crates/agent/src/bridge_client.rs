use shared::bridge::{BridgeCommand, BridgeResponse, BuildResult};
use shared::job::Job;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Spawne le com-bridge (x86) et exécute un job Build.
/// Le com-bridge est un binaire séparé compilé en i686-pc-windows-msvc.
pub async fn execute_build(job: &Job) -> Result<BuildResult, Error> {
    let bridge_exe = std::env::var("COM_BRIDGE_PATH").unwrap_or_else(|_| "com-bridge.exe".into());
    let project_path = std::env::var("PROJECT_PATH").unwrap_or_else(|_| "C:\\project.stu".into());

    let mut child = Command::new(&bridge_exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdin = child.stdin.as_mut().expect("stdin captured");
    let stdout = child.stdout.take().expect("stdout captured");
    let mut lines = BufReader::new(stdout).lines();

    send(stdin, &BridgeCommand::Ping).await?;
    send(stdin, &BridgeCommand::OpenProject { path: project_path }).await?;
    send(stdin, &BridgeCommand::Build { job_id: job.id }).await?;
    send(stdin, &BridgeCommand::CloseProject).await?;

    let mut build_result: Option<BuildResult> = None;

    while let Some(line) = lines.next_line().await? {
        let resp: BridgeResponse = serde_json::from_str(&line)?;
        match resp {
            BridgeResponse::BuildResult(r) => build_result = Some(r),
            BridgeResponse::Error { message } => return Err(message.into()),
            BridgeResponse::ProjectClosed => break,
            _ => {}
        }
    }

    child.wait().await?;

    build_result.ok_or_else(|| "com-bridge returned no BuildResult".into())
}

async fn send(stdin: &mut tokio::process::ChildStdin, cmd: &BridgeCommand) -> Result<(), Error> {
    let mut line = serde_json::to_string(cmd)?;
    line.push('\n');
    stdin.write_all(line.as_bytes()).await?;
    Ok(())
}
