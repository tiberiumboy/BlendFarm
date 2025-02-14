use super::computer_spec::ComputerSpec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Received error from database: {0}")]
    Database(String),
}

// we will use this to store data into database at some point.
#[derive(Serialize, Deserialize)]
pub struct Worker {
    pub machine_id: String,
    pub spec: ComputerSpec,
}

impl Worker {
    pub fn new(machine_id: String, spec: ComputerSpec) -> Self {
        Self { machine_id, spec }
    }
}
