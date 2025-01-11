use crate::{models::job::Job, domains::task_store::TaskError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid; 

#[derive(Debug, Serialize, Deserialize, Error)]
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
}

#[async_trait::async_trait]
pub trait JobStore {
    async fn add_job(&mut self, job: Job) -> Result<(),JobError> ;
    async fn update_job(&mut self, job: Job) -> Result<(), JobError>;
    async fn list_all(&self) -> Result<Vec<Job>, JobError>;
    async fn delete_job(&mut self, id: Uuid) -> Result<(), JobError>;
}
