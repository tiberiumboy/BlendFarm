use std::fmt::Display;

use crate::models::ticket::{CreatedTicketDto, Ticket};
use blender::{blender::BlenderError, manager::ManagerError};
use uuid::Uuid;

#[derive(Debug)]
pub enum TicketError {
    Unknown,
    DatabaseError(sqlx::Error),
    Manager(ManagerError),
    BlenderError(BlenderError),
    CacheError,
}

impl Display for TicketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketError::Unknown => f.write_str("Unknown! How did you do this?"),
            TicketError::DatabaseError(error) => {
                f.write_str(&format!("Received Database error: {error:?}"))
            }
            TicketError::Manager(manager_error) => {
                f.write_str(&format!("Received Manager error: {manager_error:?}"))
            }
            TicketError::BlenderError(blender_error) => {
                f.write_str(&format!("Received Blender error: {blender_error:?}"))
            }
            TicketError::CacheError => f.write_str("Received cache error!"),
        }
    }
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
