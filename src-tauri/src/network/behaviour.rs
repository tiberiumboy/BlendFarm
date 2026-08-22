use libp2p::{kad, swarm::NetworkBehaviour};
use crate::network::{file_request::FileRequest, file_response::FileResponse};

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    pub request_response: request_response::cbor::Behaviour<FileRequest, FileResponse>,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}