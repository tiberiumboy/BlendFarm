use crate::models::ticket::{CreatedTaskDto, Ticket};
use blender::{blender::BlenderError, manager::ManagerError};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TicketError {
    #[error("Unknown")]
    Unknown,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Manager Error: {0}")]
    Manager(#[from] ManagerError),
    #[error("Something wring with blender: {0}")]
    BlenderError(#[from] BlenderError),
    #[error("Unable to get temp storage location")]
    CacheError,
}

#[async_trait::async_trait]
pub trait TicketStore {
    // append new task to queue
    async fn add_task(&self, task: Ticket) -> Result<CreatedTaskDto, TicketError>;
    // Poll task will pop task entry from database
    async fn poll_ticket(&self) -> Result<Option<CreatedTaskDto>, TicketError>;
    // List pending task
    async fn list_tickets(&self) -> Result<Option<Vec<CreatedTaskDto>>, TicketError>;
    // delete task by id
    async fn delete_ticket(&self, id: &Uuid) -> Result<(), TicketError>;
    // delete all task with matching job id
    async fn delete_job_ticket(&self, job_id: &Uuid) -> Result<(), TicketError>;
}
