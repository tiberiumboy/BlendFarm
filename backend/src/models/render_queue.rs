use super::{project_file::ProjectFile, render_info::RenderInfo, server_setting::ServerSetting};
use blender::models::{args::Args, mode::Mode};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Io error raised: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to render: {0}")]
    BlenderError(#[from] blender::blender::BlenderError),
}

// maybe it 's best to split up? Let me take a look into traits? How can I use traits effectively?
// we will use this render queue to inform node minimal info as possible.
#[derive(Debug, Serialize, Deserialize)]
pub struct RenderQueue {
    pub frame: i32,
    pub version: Version,
    pub project_file: ProjectFile,
    pub job_id: Uuid,
}

impl RenderQueue {
    pub fn new(frame: i32, version: Version, project_file: ProjectFile, job_id: Uuid) -> Self {
        Self {
            frame,
            version,
            project_file,
            job_id,
        }
    }

    pub fn run(&self) -> Result<RenderInfo, RenderError> {
        let mut config = ServerSetting::load();
        let args = Args::new(
            self.project_file.src.clone(),
            config.render_dir.clone(),
            Mode::Frame(self.frame),
        );

        let blender = config.get_blender(self.version.clone());

        match blender.render(&args) {
            Ok(file_str) => {
                let path = PathBuf::from(file_str);
                let info = RenderInfo::new(self.frame, &path);
                Ok(info)
            }
            Err(e) => Err(RenderError::BlenderError(e)),
        }
    }
}
