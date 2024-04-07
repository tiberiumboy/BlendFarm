use std::path::PathBuf;

use super::project_file::ProjectFile;
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

#[allow(dead_code)]
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
}
