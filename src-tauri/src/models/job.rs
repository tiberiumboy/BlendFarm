/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use super::task::Task;
use crate::domains::job_store::JobError;
use blender::models::mode::Mode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{hash::Hash, path::PathBuf};
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

// This job is created by the manager and will be used to help determine the individual task created for the workers
// we will derive this job into separate task for individual workers to process based on chunk size.
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Job {
    /// Unique job identifier
    pub id: Uuid,
    /// contains the information to specify the kind of job to render (We could auto fill this from blender peek function?)
    pub mode: Mode,
    /// Path to blender files
    pub project_file: PathBuf,
    // target blender version
    pub blender_version: Version,
    // target output destination
    pub output: PathBuf,
    // completed render data.
    // TODO: discuss this? Let's map this out and see how we can better utilize this structure?
    renders: HashMap<Frame, PathBuf>,
}

impl Job {
    /// Create a new job entry with provided all information intact. Used for holding database records
    pub fn new(
        id: Uuid,
        mode: Mode,
        project_file: PathBuf,
        blender_version: Version,
        output: PathBuf,
        renders: HashMap<Frame, PathBuf>,
    ) -> Self {
        Self {
            id,
            mode,
            project_file,
            blender_version,
            output,
            renders,
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
            id: Uuid::new_v4(),
            mode,
            project_file,
            blender_version,
            output,
            renders: Default::default(),
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

impl AsRef<Uuid> for Job {
    fn as_ref(&self) -> &Uuid {
        &self.id
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Job {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
