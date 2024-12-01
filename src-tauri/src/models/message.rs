use std::path::PathBuf;

use libp2p::{futures::channel::oneshot, request_response::ResponseChannel, PeerId};
use serde::{Deserialize, Serialize};
use std::error::Error;
use thiserror::Error;
use uuid::Uuid;

use super::job::Job;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Not Connected")]
    NotConnected,
    #[error("Invalid connection")]
    Invalid,
    #[error("Bad Input")]
    BadInput,
    #[error("Send Error: {0}")]
    SendError(String),
}

// TODO: Extract this into separate file
#[derive(Debug)]
pub enum Command {
    StartJob(Job),
    FrameCompleted(PathBuf, i32),
    EndJob {
        job_id: Uuid,
    },
    Status(String),
    RequestFile {
        file_name: String,
        peer: PeerId,
        sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    },
    RespondFile {
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    },
}

// TODO: Extract this into separate file
#[derive(Debug, Serialize, Deserialize)]
pub enum NetEvent {
    // Greet () // share machine spec (cpu, gpu, ram)
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM usage realtime)
    Render(Job),
    // think I need to send this somewhere else.
    // SendFile { file_name: String, data: Vec<u8> },
    Status(String),
    NodeDiscovered(String),
    NodeDisconnected(String),
}

// TODO: Learn about macro rules? This would be a great substitution for meta programming
impl NetEvent {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileRequest(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileResponse(Vec<u8>);
