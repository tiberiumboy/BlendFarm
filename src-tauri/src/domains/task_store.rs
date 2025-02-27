use crate::models::task::{CreatedTaskDto, NewTaskDto};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, Serialize, Deserialize)]
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
    async fn add_task(&self, task: NewTaskDto) -> Result<CreatedTaskDto, TaskError>;
    // Poll task will pop task entry from database
    async fn poll_task(&self) -> Result<CreatedTaskDto, TaskError>;
    // delete task by id
    async fn delete_task(&self, id: &Uuid) -> Result<(), TaskError>;
    // delete all task with matching job id
    async fn delete_job_task(&self, job_id: &Uuid) -> Result<(), TaskError>;
}
