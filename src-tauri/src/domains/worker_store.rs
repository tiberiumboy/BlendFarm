use crate::models::worker::Worker;

#[derive(Debug)]
pub enum WorkerError {
    Database(String),
}

// TOOD: Reimplement unique identifier - Had to remove Peer_id, part of purging libp2p in favor for iroh crate.
#[async_trait::async_trait]
pub trait WorkerStore {
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError>;
    async fn get_worker(&self) -> Option<Worker>;
    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError>;
    async fn delete_worker(&mut self) -> Result<(), WorkerError>;
    async fn clear_worker(&mut self) -> Result<(), WorkerError>;
}
