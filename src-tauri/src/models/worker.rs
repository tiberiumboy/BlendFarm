use super::computer_spec::ComputerSpec;
// use crate::services::server::ServerEvent;
use libp2p::PeerId;

// Treat this struct as server found on network
#[derive(Debug)]
pub struct Worker {
    pub peer_id: PeerId,
    pub spec: ComputerSpec,
    // internally, we should at least documented the logs and entry.
    // logs: Vec<ServerEvent>,
}

impl Worker {
    pub fn new(peer_id: PeerId, spec: ComputerSpec) -> Self {
        Self {
            peer_id,
            spec,
            /*logs: Vec::new()*/
        }
    }
}
