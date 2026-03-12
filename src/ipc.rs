use peercred_ipc::Client;
use serde::{Deserialize, Serialize};

fn control_socket_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{home}/.claude/orchestrator/control.sock")
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ControlRequest {
    SendMessage {
        project: String,
        to: String,
        content: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ControlResponse {
    Ok,
    Error {
        message: String,
    },
    #[serde(other)]
    Unknown,
}

pub fn send_message(project: &str, to: &str, content: &str) -> Result<(), String> {
    let response: ControlResponse = Client::call(
        &control_socket_path(),
        &ControlRequest::SendMessage {
            project: project.to_string(),
            to: to.to_string(),
            content: content.to_string(),
        },
    )
    .map_err(|e| format!("IPC error: {e}"))?;

    match response {
        ControlResponse::Ok => Ok(()),
        ControlResponse::Error { message } => Err(message),
        ControlResponse::Unknown => Ok(()),
    }
}
