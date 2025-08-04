use super::job::JobEvent;
use super::{behaviour::FileResponse, network::NodeEvent};
use futures::channel::oneshot::{self};
use libp2p::PeerId;
use libp2p_request_response::{OutboundRequestId, ResponseChannel};
use std::path::PathBuf;
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
    #[error("Timeout, unable to connect peer")]
    Timeout,
}

pub type KeywordSearch = String;

// to make things simple, we'll create a file service command to handle file service.
#[derive(Debug)]
pub enum FileCommand {
    StartProviding(KeywordSearch, PathBuf), // update kademlia service to provide a new file. Must have a file name and a extension! Cannot be a directory!
    StopProviding(KeywordSearch),           // update kademlia service to stop providing the file.
    GetProviders {
        file_name: String,
        sender: oneshot::Sender<Option<HashSet<PeerId>>>,
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
    RequestFilePath {
        keyword: KeywordSearch,
        sender: oneshot::Sender<Option<PathBuf>>,
    },
}

// Send commands to network.
#[derive(Debug)]
pub enum Command {
    NodeStatus(NodeEvent), // broadcast node activity changed
    JobStatus(JobEvent),
    FileService(FileCommand),
}

// Received network events.
#[derive(Debug)]
pub enum Event {
    // Don't think I need this anymore, trying to rely on DHT for node availability somehow?
    // TODO: See about utilizing DHT instead of this? How can I get event from DHT?
    NodeStatus(NodeEvent),
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
    JobUpdate(JobEvent),
    ReceivedFileData(OutboundRequestId, Vec<u8>),
}
