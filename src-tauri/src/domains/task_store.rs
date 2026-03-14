use crate::models::task::{CreatedTaskDto, Task};
use blender::blender::BlenderError;
// use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)] // Serialize, Deserialize
pub enum TaskError {
    #[error("Unknown")]
    Unknown,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Something wring with blender: {0}")]
    BlenderError(#[from] BlenderError),
    #[error("Unable to get temp storage location")]
    CacheError,
}

#[async_trait::async_trait]
pub trait TaskStore {
    // append new task to queue
    async fn add_task(&self, task: Task) -> Result<CreatedTaskDto, TaskError>;
    // Poll task will pop task entry from database
    async fn poll_task(&self) -> Result<Option<CreatedTaskDto>, TaskError>;
    // List pending task
    async fn list_tasks(&self) -> Result<Option<Vec<CreatedTaskDto>>, TaskError>;
    // delete task by id
    async fn delete_task(&self, id: &Uuid) -> Result<(), TaskError>;
    // delete all task with matching job id
    async fn delete_job_task(&self, job_id: &Uuid) -> Result<(), TaskError>;
}
