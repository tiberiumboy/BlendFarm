use libp2p::{
    gossipsub::{self},
    kad::{self},
    mdns,
    swarm::NetworkBehaviour,
};
use libp2p_request_response::cbor;

use crate::network::{file_request::FileRequest, file_response::FileResponse};

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    // file transfer response protocol
    pub request_response: cbor::Behaviour<FileRequest, FileResponse>,

    // broadcast message to listening node (chat relay)
    pub gossipsub: gossipsub::Behaviour,

    // self discovery network service
    pub mdns: mdns::tokio::Behaviour,

    // used to provide file availability
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}
