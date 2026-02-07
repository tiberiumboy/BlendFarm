use libp2p::{
    gossipsub::{self},
    kad::{self},
    mdns,
    swarm::NetworkBehaviour,
};
use libp2p_request_response::cbor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRequest(pub String);

// may be changed to use stream?
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
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

