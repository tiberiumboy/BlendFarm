use futures::channel::oneshot::{self};
use libp2p::{Multiaddr, PeerId};
use libp2p_request_response::{OutboundRequestId, ResponseChannel};
use std::path::PathBuf;
use std::{collections::HashSet, error::Error};
use thiserror::Error;

use crate::models::behaviour::FileResponse;
use crate::models::job::JobEvent;
use crate::services::server::ServerEvent;

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

// TODO: Find a way to handle errors properly
pub type CommandError = Box<dyn Error + Send>;

// TODO: Find a way to cast this as FileStruct?
pub type FileData = Vec<u8>;

pub type FileResult<T> = Result<T, CommandError>;

// to make things simple, we'll create a file service command to handle file service.
#[derive(Debug)]
pub enum FileCommand {
    Dial {
        peer_addr: Multiaddr,
        sender: oneshot::Sender<FileResult<()>>,
    },
    // TODO: Find a way to get around the string type! This expects a copy!
    StartProviding {
        file_name: KeywordSearch,
        sender: oneshot::Sender<()>,
    },
    // StartProviding(KeywordSearch, PathBuf), // update kademlia service to provide a new file. Must have a file name and a extension! Cannot be a directory!
    StopProviding(KeywordSearch),           // update kademlia service to stop providing the file.
    GetProviders {
        file_name: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestFile {
        peer_id: PeerId,
        file_name: String,
        sender: oneshot::Sender<FileResult<FileData>>,
    },
    RespondFile {
        file: FileData,
        channel: ResponseChannel<FileResponse>,
    },
    RequestFilePath {
        keyword: KeywordSearch,
        sender: oneshot::Sender<Option<PathBuf>>,
    },
}

// Send commands to network.
// TODO: Make two different kind of message, one use for broadcast and the other for direct communication.
#[derive(Debug)]
pub enum Command {
    Subscribe {
        topic: String,
    },
    // Keep this command here instead of FileCommand and use this as general command instead?
    StartListening {
        addr: Multiaddr,
        sender: oneshot::Sender<FileResult<()>>,
    },
    StopListening,
    // Message this peer with server events. (Consider looking into receiving NetworkRequest enum?)
    // These are signal to use to send out message and forget.
    // May receive a response back, using the direct message above.
    Message(Option<Multiaddr>, ServerEvent),
    FileService(FileCommand),
}

// Received network events.
#[derive(Debug)]
pub enum Event {
    // When the node becomes available on the network.
    Discovered(PeerId, Multiaddr),
    // this is used for file transfer protocol
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
    // may not actually need this?
    ServerStatus(ServerEvent),
    // may not actually need this?
    JobUpdate(JobEvent),
    // Used to exchange file data
    ReceivedFileData(OutboundRequestId, Vec<u8>),
}
