use super::computer_spec::ComputerSpec;
use libp2p::PeerId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Received error from database: {0}")]
    Database(String),
}

#[derive(Debug)]
pub struct Worker {
    // machine id is really just peer_id
    pub machine_id: PeerId,
    pub spec: ComputerSpec,
}

impl Worker {
    pub fn new(machine_id: PeerId, spec: ComputerSpec) -> Self {
        Self { machine_id, spec }
    }
}