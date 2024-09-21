use std::path::Path;

use super::{project_file::ProjectFile, render_info::RenderInfo};
use blender::manager::Manager as BlenderManager;
use blender::models::{args::Args, mode::Mode, status::Status};
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Unable to complete command! Program unexpectly crashed and closed pipe?")]
    BrokenPipe,
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

    pub fn run(&self, output: impl AsRef<Path>) -> Result<RenderInfo, RenderError> {
        let args = Args::new(
            self.project_file.file_path(),
            // TODO: find a better way to handle target destination for render outputs
            output,
            Mode::Frame(self.frame),
        );

        let mut manager = BlenderManager::load();
        let blender = manager.fetch_blender(&self.version).unwrap();
        let listener = blender.render(args);

        while let Ok(event) = listener.recv() {
            match event {
                Status::Completed { result } => {
                    let info = RenderInfo::new(self.frame, &result);
                    return Ok(info);
                }
                Status::Error(e) => return Err(RenderError::BlenderError(e)),
                _ => {} //
            }
        }
        Err(RenderError::BrokenPipe)
    }
}
