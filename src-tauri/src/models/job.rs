/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use blender::blender::{Blender, BlenderError, Manager};
use blender::models::{args::Args, mode::Mode, status::Status};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    hash::Hash,
    path::PathBuf,
};
use thiserror::Error;
use uuid::Uuid;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("Job failed to run: {0}")]
    FailedToRun(String),
    // it would be nice to have blender errors here?
    #[error("Invalid blend file: {0}")]
    InvalidFile(String),
}

// pub trait JobStatus {}
#[derive(Debug)]
pub enum JobStatus {
    /// Job is idle - Do we need this?
    Idle,
    /// Pause the working job, (cancel blender process, and wait for incoming packet)
    Paused,
    Downloading(String),
    // find a way to parse output data, and provide percentage of completion here
    /// percentage of completion
    Running {
        frame: f32,
    },
    Error(JobError),
    /// The job has been completed
    Completed,
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

    pub fn get_file_name(&self) -> Option<&str> {
        match self.project_file.file_name() {
            Some(v) => v.to_str(),
            None => None
        }
    }

    // TODO: consider about how I can invoke this command from network protocol?
    // Invoke blender to run the job
    // Find out if I need to run this locally, or just rely on the server to perform the operation?
    pub async fn run(&mut self, output: PathBuf) -> Result<std::sync::mpsc::Receiver<Status>, JobError> {    
        
        let file = self.project_file.clone();
        let mode = self.mode.clone();
        let args = Args::new(file, output, mode);

        // TODO: how can I ask peers for copy of blender?
        let mut manager = Manager::load();
        let blender = manager.fetch_blender(&self.blender_version).map_err(|e| JobError::FailedToRun(e.to_string()))?;

        // here's the question - how do I send the host the image of the completed rendered job? topic? provider?
        let receiver = blender.render(args).await;
        Ok(receiver)
    }

    pub fn set_project_file(mut self, project_file: PathBuf) -> Self {
        self.project_file = project_file;
        self
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
