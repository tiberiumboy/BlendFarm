use super::behaviour::FileResponse;
use super::computer_spec::ComputerSpec;
use super::job::JobEvent;
use futures::channel::oneshot;
use libp2p::PeerId;
use libp2p_request_response::ResponseChannel;
use std::{collections::HashSet, error::Error};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Unable to listen: {0}")]
    UnableToListen(String),
    #[error("Not Connected")]
    NotConnected,
    #[error("Invalid connection")]
    Invalid,
    #[error("Bad Input")]
    BadInput,
    #[error("Send Error: {0}")]
    SendError(String),
    #[error("No peers on network have this file available to download!")]
    NoPeerProviderFound,
    #[error("Unable to save download file: {0}")]
    UnableToSave(String),
}

// Send commands to network.
#[derive(Debug)]
pub enum NetCommand {
    SendIdentity,
    Status(String),
    SubscribeTopic(String),
    UnsubscribeTopic(String),
    JobStatus(PeerId, JobEvent),
    // use this event to send message to a specific node
    StartProviding {
        file_name: String,
        // path: PathBuf,
        sender: oneshot::Sender<()>,
    },
    GetProviders {
        file_name: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestFile {
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
    OnConnected,
    NodeDiscovered(PeerId, ComputerSpec), 
    // TODO: Future impl. Use this to send computer activity
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM activity readings)
    NodeDisconnected(PeerId), // On Node disconnected
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
    JobUpdate(JobEvent),
}
