use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::{computer_spec::ComputerSpec, job::Job};

// TODO figure out what I was doing here?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileRequest(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResponse(Vec<u8>);

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

// Send commands to network.
#[derive(Debug)]
pub enum NetCommand {
    StartJob(Job),
    FrameCompleted(PathBuf, i32),
    EndJob { job_id: Uuid },
    Status(String),
    // RequestFile {
    //     file_name: String,
    //     peer: PeerId,
    //     sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    // },
    // RespondFile {
    //     file: Vec<u8>,
    //     channel: ResponseChannel<FileResponse>,
    // },
    SendIdentity,
    Shutdown,
}

// TODO: Received network events.
#[derive(Debug, Serialize, Deserialize)]
pub enum NetEvent {
    // Share basic computer configuration for sharing Blender compatible executable over the network. (To help speed up the installation over the network.)
    Identity(String, ComputerSpec),
    // TODO: Future impl. Use this to send computer activity
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM activity readings)
    Render(Job),              // Receive a new render job
    Status(String, String), // Receive message status (To GUI?) Could I treat this like Chat messages?
    NodeDiscovered(String), // On Node discover
    NodeDisconnected(String), // On Node disconnected
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
