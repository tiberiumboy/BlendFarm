/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use super::task::Task;
use super::with_id::WithId;
use crate::domains::job_store::JobError;
use blender::models::mode::Mode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum JobEvent {
    Render(Task),
    Remove(Uuid),
    RequestTask,
    ImageCompleted {
        job_id: Uuid,
        frame: Frame,
        file_name: String,
    },
    JobComplete,
    Error(JobError),
}

pub type Frame = i32;
pub type NewJobDto = Job;
pub type CreatedJobDto = WithId<Job, Uuid>;

// This job is created by the manager and will be used to help determine the individual task created for the workers
// we will derive this job into separate task for individual workers to process based on chunk size.
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Job {
    /// contains the information to specify the kind of job to render (We could auto fill this from blender peek function?)
    pub mode: Mode,
    /// Path to blender files
    pub project_file: PathBuf,
    // target blender version
    pub blender_version: Version,
    // target output destination
    pub output: PathBuf,
}

impl Job {
    /// Create a new job entry with provided all information intact. Used for holding database records
    pub fn new(
        mode: Mode,
        project_file: PathBuf,
        blender_version: Version,
        output: PathBuf,
    ) -> Self {
        Self {
            mode,
            project_file,
            blender_version,
            output,
        }
    }

    /// Create a new job entry from the following parameter inputs
    pub fn from(
        project_file: PathBuf,
        output: PathBuf,
        blender_version: Version,
        mode: Mode,
    ) -> Self {
        Self {
            mode,
            project_file,
            blender_version,
            output,
        }
    }

    pub fn get_file_name(&self) -> &str {
        self.project_file.file_name().unwrap().to_str().unwrap()
    }

    pub fn get_project_path(&self) -> &PathBuf {
        &self.project_file
    }

    pub fn get_version(&self) -> &Version {
        &self.blender_version
    }
}
