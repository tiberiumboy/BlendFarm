use super::with_id::WithId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub type CreatedRenderInfoDto = WithId<RenderInfo, Uuid>;
pub type NewRenderInfoDto = RenderInfo;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct RenderInfo {
    pub job_id: Uuid,
    pub frame: i32,
    pub render_path: PathBuf,
}

impl RenderInfo {
    pub fn new(job_id: Uuid, frame: i32, path: impl AsRef<Path>) -> Self {
        Self {
            job_id,
            frame,
            render_path: path.as_ref().to_path_buf(),
        }
    }
}
