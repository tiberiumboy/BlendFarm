use futures::channel::oneshot;
use libp2p::{
    gossipsub::{self},
    kad::{self},
    mdns, ping,
    swarm::NetworkBehaviour,
    PeerId,
};
use libp2p_request_response::{cbor, OutboundRequestId};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRequest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResponse(pub Vec<u8>);

#[derive(Default, Debug)]
pub struct FileService {
    // I am still trying to figure out what to do with this...
    pub providing_files: HashMap<String, PathBuf>,
    pub pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pub pending_start_providing: HashMap<kad::QueryId, oneshot::Sender<()>>,
    pub pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

impl FileService {
    pub fn new() -> Self {
        FileService {
            providing_files: HashMap::new(),
            pending_get_providers: HashMap::new(),
            pending_start_providing: HashMap::new(),
            pending_request_file: HashMap::new(),
        }
    }

    // impl. a load function which populates providing files based on given rules/schema.
}

#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
    // to ping node for responsiveness and activity
    pub ping: ping::Behaviour,
    // file transfer response protocol
    pub request_response: cbor::Behaviour<FileRequest, FileResponse>,
    // Communication between peers to pepers
    pub gossipsub: gossipsub::Behaviour,
    // self discovery network service
    pub mdns: mdns::tokio::Behaviour,
    // used to provide file availability
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
}

// would this work for me?
impl BlendFarmBehaviour {}
