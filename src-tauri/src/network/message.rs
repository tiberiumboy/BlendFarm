use blender::models::event::BlenderEvent;
use futures::channel::oneshot::{self};
use libp2p::gossipsub::TopicHash;
use libp2p::{Multiaddr, PeerId};
use libp2p_request_response::{OutboundRequestId, ResponseChannel};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{collections::HashSet, error::Error};
use thiserror::Error;

use crate::models::behaviour::FileResponse;
use crate::models::computer_spec::ComputerSpec;
use crate::models::job::JobEvent;
use crate::network::PeerIdString;

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
    RequestFilePath {
        keyword: KeywordSearch,
        sender: oneshot::Sender<Option<PathBuf>>,
    },
}

// Send commands to network.
#[derive(Debug)]
pub enum Command {
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    Subscribe {
        topic: String,
    },
    // TODO: figure out a way to get around the Box<dyn Error + Send> traits!
    StartListening {
        addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    // TODO: Find a way to get around the string type! This expects a copy!
    StartProviding {
        file_name: String,
        sender: oneshot::Sender<()>,
    },

    GetProviders {
        file_name: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestFile {
        file_name: String,
        peer: PeerId,
        sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    },
    RespondFile {
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    },

    // TODO: More documentation to explain below
    // These are signal to use to send out message and forget.
    // May expect a respoonse back potentially requesting this node to work new jobs.
    NodeStatus(NodeEvent), // broadcast node activity changed
    JobStatus(JobEvent),
    FileService(FileCommand),
}

// Must be serializable to send data across network
// issue with this is that this cannot be convert into Encode,Decode by bincode. Instead we'll have to
#[derive(Debug, Serialize, Deserialize)]
pub enum NodeEvent {
    Hello(PeerIdString, ComputerSpec),
    Disconnected {
        peer_id: PeerIdString,
        reason: Option<String>,
    },
    BlenderStatus(BlenderEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusEvent {
    Offline,
    Online,
    Busy,
    Error(String),
    Signal(String),
}

#[derive(Debug)]
pub(crate) enum ChannelStatus {
    Joined(PeerId, TopicHash),
    Disconnected(PeerId, TopicHash),
}

// Received network events.
#[derive(Debug)]
pub enum Event {
    // Don't think I need this anymore, trying to rely on DHT for node availability somehow?
    // TODO: See about utilizing DHT instead of this? How can I get event from DHT?
    Discovered(PeerId, Multiaddr),
    Channel(ChannelStatus),
    NodeStatus(NodeEvent),
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
    JobUpdate(JobEvent),
    ReceivedFileData(OutboundRequestId, Vec<u8>),
}
