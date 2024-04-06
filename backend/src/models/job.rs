use std::path::PathBuf;

use super::project_file::ProjectFile;
use crate::services::blender::render;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JobStatus {
    Idle,
    Running,
    Done,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub project_file: ProjectFile,
    pub output: PathBuf,
    pub status: JobStatus,
    pub created_at: String,
}

impl Job {
    pub fn new(project_file: ProjectFile) -> Job {
        Job {
            id: Uuid::new_v4().to_string(),
            project_file,
            output: PathBuf::new(),
            status: JobStatus::Idle,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn run(&mut self) {
        self.status = JobStatus::Running;

        self.project_file.move_to_temp();

        if let Some(tmp) = &self.project_file.tmp {
            let _output = render(&self, 0).unwrap();
            // if we're the nodes, we need to send the image back to the host.
        }

        self.project_file.clear_temp();

        // Run the job
        self.status = JobStatus::Done;
    }

    pub fn source(&self) -> &str {
        self.project_file
            .tmp
            .or_else(|| Some(self.project_file.src))
            .expect("Missing path!")
            .to_str()
            .unwrap()
    }
}
