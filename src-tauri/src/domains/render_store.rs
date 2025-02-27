use crate::models::render_info::{CreatedRenderInfoDto, NewRenderInfoDto, RenderInfo};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Missing file")]
    MissingFileAtPath,
    #[error("Database Errors")]
    DatabaseError(String),
}

#[async_trait::async_trait]
pub trait RenderStore {
    async fn list_renders(&self) -> Result<Vec<CreatedRenderInfoDto>, RenderError>;
    async fn create_renders(
        &self,
        render_info: NewRenderInfoDto,
    ) -> Result<CreatedRenderInfoDto, RenderError>;
    async fn read_renders(&self, id: &Uuid) -> Result<CreatedRenderInfoDto, RenderError>;
    async fn update_renders(&mut self, render_info: RenderInfo) -> Result<(), RenderError>;
    async fn delete_renders(&mut self, id: &Uuid) -> Result<(), RenderError>;
}
