use std::str::FromStr;
use super::{computer_spec::ComputerSpec, network::PeerIdString};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Worker {
    pub id: PeerIdString,
    #[sqlx(json)]
    pub spec: ComputerSpec,
}

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Received error from database: {0}")]
    Database(String),
}

impl Worker {
    pub fn new(id: PeerIdString, spec: ComputerSpec) -> Self {
        Self {
            id,
            spec
        }
    }

    // not in use?
    pub fn peer_id(self) -> PeerId {
        PeerId::from_str(&self.id).expect("Should not fail?")
    }
}