use crate::models::ticket::{CreatedTicketDto, Ticket};
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
    // append new ticket to queue
    async fn add_ticket(&self, ticket: Ticket) -> Result<CreatedTicketDto, TicketError>;
    // Poll ticket will pop ticket entry from database
    async fn poll_ticket(&self) -> Result<Option<CreatedTicketDto>, TicketError>;
    // List pending ticket
    async fn list_tickets(&self) -> Result<Option<Vec<CreatedTicketDto>>, TicketError>;
    // delete ticket by id
    async fn delete_ticket(&self, id: &Uuid) -> Result<(), TicketError>;
    // delete all ticket with matching job id
    async fn delete_job_ticket(&self, job_id: &Uuid) -> Result<(), TicketError>;
}
