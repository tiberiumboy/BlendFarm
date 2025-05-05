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

#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
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
