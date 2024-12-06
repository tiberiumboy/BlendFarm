use libp2p::{gossipsub, kad, mdns, swarm::NetworkBehaviour};
use libp2p_request_response::cbor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRequest(String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResponse(Vec<u8>);

#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
    pub request_response: cbor::Behaviour<FileRequest, FileResponse>,
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub kad: kad::Behaviour<kad::store::MemoryStore>, // Ok so I need to figure out how this works? Figure out about TStore trait
}
