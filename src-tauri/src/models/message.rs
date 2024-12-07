use std::path::PathBuf;
use std::{collections::HashSet, error::Error};

use futures::channel::oneshot;
use libp2p::PeerId;
use libp2p_request_response::ResponseChannel;
use thiserror::Error;
use uuid::Uuid;

use super::behaviour::FileResponse;
use super::{computer_spec::ComputerSpec, job::Job};

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
    EndJob {
        job_id: Uuid,
    },
    Status(String),
    StartProviding {
        file_name: String,
        sender: oneshot::Sender<()>,
    },
    GetProviders {
        file_name: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    SendIdentity,
    RequestFile {
        // ack?
        peer_id: PeerId,
        file_name: String,
        sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    },
    RespondFile {
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    },
}

// TODO: Received network events.
#[derive(Debug)]
pub enum NetEvent {
    // Share basic computer configuration for sharing Blender compatible executable over the network. (To help speed up the installation over the network.)
    Status(PeerId, String), // Receive message status (To GUI?) Could I treat this like Chat messages?
    Identity(PeerId, ComputerSpec),
    // TODO: Future impl. Use this to send computer activity
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM activity readings)
    Render(PeerId, Job),      // Receive a new render job
    NodeDiscovered(String),   // On Node discover
    NodeDisconnected(String), // On Node disconnected
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
}
