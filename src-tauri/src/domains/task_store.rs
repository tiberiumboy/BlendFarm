use crate::models::task::Task;
use libp2p::PeerId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Unknown")]
    Unknown,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[async_trait::async_trait]
pub trait TaskStore {
    // append new task to queue
    async fn add_task(&mut self, requestor: PeerId, task: Task) -> Result<(), TaskError>;
    // Poll task will pop task entry from database
    async fn poll_task(&mut self) -> Result<(PeerId, Task), TaskError>;
    // delete task by id
    async fn delete_task(&mut self, task:Task) -> Result<(), TaskError>;
}