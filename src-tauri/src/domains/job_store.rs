use std::fmt::Display;

use crate::{
    domains::ticket_store::TicketError,
    models::job::{CreatedJobDto, NewJobDto},
};
use blender::blender::BlenderError;
// use serde::{Deserialize, Serialize};
// use thiserror::Error;
use uuid::Uuid;

#[derive(Debug)] // Error, Serialize, Deserialize, 
pub enum JobError {
    // #[error("Job failed to run: {0}")]
    FailedToRun(String),
    // #[error("Invalid blend file: {0}")]
    InvalidFile(String),
    // #[error("Received Database errors! {0}")]
    DatabaseError(String),
    // #[error("Task error")]
    TicketError(
        // #[from]
        TicketError,
    ),
    // #[error("Command error: {0}")]
    Send(String),
    Blender(BlenderError),
}

impl Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobError::FailedToRun(message) => f.write_str(&format!("Job failed to run: {message}")),
            JobError::InvalidFile(message) => {
                f.write_str(&format!("Invalid blend file: {message}"))
            }
            JobError::DatabaseError(error) => {
                f.write_str(&format!("Received Database errors! {error:?}"))
            }
            JobError::TicketError(ticket_error) => {
                f.write_str(&format!("Ticket error: {ticket_error:?}"))
            }
            JobError::Send(command) => f.write_str(&format!("Command error: {command:?}")),
            JobError::Blender(blender_error) => {
                f.write_str(&format!("Received blender error! {blender_error:?}"))
            }
        }
    }
}

#[async_trait::async_trait]
pub trait JobStore {
    async fn add_job(&mut self, job: NewJobDto) -> Result<CreatedJobDto, JobError>;
    async fn list_all(&self) -> Result<Vec<CreatedJobDto>, JobError>;
    async fn get_job(&self, job_id: &Uuid) -> Result<Option<CreatedJobDto>, JobError>;
    async fn update_job(&mut self, job: CreatedJobDto) -> Result<(), JobError>;
    async fn delete_job(&mut self, id: &Uuid) -> Result<(), JobError>;
}
