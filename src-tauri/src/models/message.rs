use super::{behaviour::FileResponse, network::NodeEvent};
// use super::computer_spec::ComputerSpec;
use super::job::JobEvent;
use std::path::PathBuf;
use futures::channel::oneshot::{self, Sender};
use libp2p::{kad::QueryId, PeerId};
use libp2p_request_response::{OutboundRequestId, ResponseChannel};
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

pub type Target = Option<String>;

// Send commands to network.
#[derive(Debug)]
pub enum Command {
    // what's the reason behind this?
    IncomingWorker(PeerId),
    Status(String),
    SubscribeTopic(String),
    UnsubscribeTopic(String),
    NodeStatus(NodeEvent), // broadcast node activity changed
    JobStatus(Target, JobEvent),
    StartProviding(PathBuf),    // update kademlia service to provide a new file. Must have a file name and a extension! Cannot be a directory!
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
pub enum Event {
    // Share basic computer configuration for sharing Blender compatible executable over the network. (To help speed up the installation over the network.)
    Status(PeerId, String), // Receive message status (To GUI?) Could I treat this like Chat messages?
    OnConnected(PeerId),
    NodeStatus(NodeEvent),
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
    JobUpdate(JobEvent),
    PendingRequestFiled(
        OutboundRequestId,
        Option<Sender<Result<Vec<u8>, Box<dyn Error + Send + 'static>>>>,
    ),
    PendingGetProvider(QueryId, Sender<HashSet<PeerId>>),
    ReceivedFileData(OutboundRequestId, Vec<u8>),

}
