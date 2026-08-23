use crate::network::{file_request::FileRequest, file_response::FileResponse};
use libp2p::{kad, swarm::NetworkBehaviour};
use libp2p_request_response::cbor::Behaviour as RequestResponseBehaviour;

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    pub request_response: RequestResponseBehaviour<FileRequest, FileResponse>,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}
