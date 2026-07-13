use shared::bridge::{BridgeCommand, BridgeResponse};
use std::io::{self, BufRead, Write};

mod com;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(e) => {
                eprintln!("stdin error: {e}");
                break;
            }
        };

        let response = match serde_json::from_str::<BridgeCommand>(&line) {
            Ok(cmd) => com::handle(cmd),
            Err(e) => BridgeResponse::Error {
                message: format!("parse error: {e}"),
            },
        };

        let mut json = serde_json::to_string(&response).unwrap();
        json.push('\n');
        out.write_all(json.as_bytes()).unwrap();
        out.flush().unwrap();
    }
}
