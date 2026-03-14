use crate::{
    domains::task_store::TaskError,
    models::job::{CreatedJobDto, NewJobDto},
};
// use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)] // Serialize, Deserialize, 
pub enum JobError {
    #[error("Job failed to run: {0}")]
    FailedToRun(String),
    // it would be nice to have blender errors here?
    #[error("Invalid blend file: {0}")]
    InvalidFile(String),
    #[error("Received Database errors! {0}")]
    DatabaseError(String),
    #[error("Task error")]
    TaskError(#[from] TaskError),
    #[error("Command error: {0}")]
    Send(String),
}

#[async_trait::async_trait]
pub trait JobStore {
    async fn add_job(&mut self, job: NewJobDto) -> Result<CreatedJobDto, JobError>;
    async fn list_all(&self) -> Result<Vec<CreatedJobDto>, JobError>;
    async fn get_job(&self, job_id: &Uuid) -> Result<Option<CreatedJobDto>, JobError>;
    async fn update_job(&mut self, job: CreatedJobDto) -> Result<(), JobError>;
    async fn delete_job(&mut self, id: &Uuid) -> Result<(), JobError>;
}
