use super::computer_spec::ComputerSpec;
use libp2p::PeerId;
use thiserror::Error;

#[derive(Debug)]
pub struct Worker {
    pub id: PeerId,
    pub spec: ComputerSpec,
}

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Received error from database: {0}")]
    Database(String),
}

impl Worker {
    pub fn new(id: PeerId, spec: ComputerSpec) -> Self {
        Self { id, spec }
    }
}
