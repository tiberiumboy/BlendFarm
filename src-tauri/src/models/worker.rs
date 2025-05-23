use super::{computer_spec::ComputerSpec, network::PeerIdString, with_id::WithId};
use thiserror::Error;

pub type Worker = WithId<ComputerSpec, PeerIdString>;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Received error from database: {0}")]
    Database(String),
}