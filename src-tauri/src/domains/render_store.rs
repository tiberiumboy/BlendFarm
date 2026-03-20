use std::{collections::HashMap, path::PathBuf};

use crate::models::{job::JobId, render_info::{CreatedRenderInfoDto, NewRenderInfoDto, RenderInfo}};
use blender::blender::Frame;
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
    async fn find(&self, filter: Option<JobId>) -> Result<HashMap<Frame, PathBuf>, RenderError>;
    async fn update(&mut self, render_info: RenderInfo) -> Result<(), RenderError>;
    async fn create(
        &self,
        render_info: NewRenderInfoDto,
    ) -> Result<CreatedRenderInfoDto, RenderError>;
    async fn kill(&mut self, id: &Uuid) -> Result<(), RenderError>;
}
