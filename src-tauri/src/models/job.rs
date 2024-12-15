/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use blender::blender::Blender;
use blender::models::{args::Args, mode::Mode, status::Status};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{hash::Hash, path::PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum JobError {
    #[error("Job failed to run: {0}")]
    FailedToRun(String),
    // it would be nice to have blender errors here?
    #[error("Invalid blend file: {0}")]
    InvalidFile(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum JobEvent {
    Render(Job),
    RequestJob,
    ImageCompleted {
        id: Uuid,
        frame: Frame,
        file_name: String,
    },
    JobComplete,
    Error(JobError),
}

pub type Frame = i32;

// how do I make this job extend it's lifespan? I need to monitor and regulate all on-going job method?
// if a node joins the server, we automatically assign a new active job to the node.
/// A container to hold rendering job information. This will be used to send off jobs to all other rendering farm
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    /// Unique job identifier
    id: Uuid,
    /// What kind of mode should this job run as
    mode: Mode,
    /// Path to blender files
    project_file: PathBuf,
    // target blender version
    blender_version: Version,
    // completed render data.
    // TODO: discuss this? Let's map this out and see how we can better utilize this structure?
    renders: HashMap<Frame, PathBuf>,
}

impl Job {
    pub fn new(project_file: PathBuf, blender_version: Version, mode: Mode) -> Job {
        Job {
            id: Uuid::new_v4(),
            project_file,
            blender_version,
            mode,
            renders: Default::default(),
        }
    }

    pub fn get_project_path(&self) -> &PathBuf {
        &self.project_file
    }

    pub fn set_project_path(mut self, new_path: PathBuf) -> Self {
        self.project_file = new_path;
        self
    }

    pub fn get_file_name(&self) -> Option<&str> {
        match self.project_file.file_name() {
            Some(v) => v.to_str(),
            None => None,
        }
    }

    pub fn get_version(&self) -> &Version {
        &self.blender_version
    }

    // Invoke blender to run the job
    pub async fn run(
        &mut self,
        output: PathBuf,
        blender: &Blender,
    ) -> Result<std::sync::mpsc::Receiver<Status>, JobError> {
        let file = self.project_file.clone();
        let mode = self.mode.clone();
        let args = Args::new(file, output, mode);

        // TODO: How can I adjust blender jobs?
        let receiver = blender.render(args).await;
        Ok(receiver)
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
