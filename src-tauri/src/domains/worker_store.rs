use libp2p::PeerId;

use crate::models::worker::{Worker, WorkerError};

#[async_trait::async_trait]
pub trait WorkerStore {
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError>;
    async fn get_worker(&self, id: &str) -> Option<Worker>;
    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError>;
    async fn delete_worker(&mut self, machine_id: &PeerId) -> Result<(), WorkerError>;
    async fn clear_worker(&mut self) -> Result<(), WorkerError>;
}
