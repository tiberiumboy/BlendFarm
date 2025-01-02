use crate::models::task::Task;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Unknown")]
    Unknown,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Something wring with blender: {0}")]
    BlenderError(String),
}

#[async_trait::async_trait]
pub trait TaskStore {
    // append new task to queue
    async fn add_task(&mut self, task: Task) -> Result<(), TaskError>;
    // Poll task will pop task entry from database
    async fn poll_task(&mut self) -> Result<Task, TaskError>;
    // delete task by id
    async fn delete_task(&mut self, task: Task) -> Result<(), TaskError>;
    // delete all task with matching job id
    async fn delete_job_task(&mut self, job_id: Uuid) -> Result<(), TaskError>;
}
