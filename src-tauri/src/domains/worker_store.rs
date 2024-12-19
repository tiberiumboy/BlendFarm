use crate::models::worker::{Worker, WorkerError};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait WorkerStore {
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError>;
    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError>;
    async fn delete_worker(&mut self, id: Uuid) -> Result<(), WorkerError>;
}
