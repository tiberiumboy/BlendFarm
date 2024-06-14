/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use super::{project_file::ProjectFile, render_node::RenderNode, server_setting::ServerSetting};
use std::{
    io::Result,
    path::{Path, PathBuf},
};
// use crate::services::sender;
use blender::{args::Args, blender::Blender, mode::Mode};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::thread::{self, JoinHandle};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum JobError {
    #[error("Job failed to run: {0}")]
    FailedToRun(String),
    // it would be nice to have blender errors here?
}

// pub trait JobStatus {}
#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct RenderInfo {
    pub frame: i32,
    pub path: PathBuf,
}

/// A container to hold rendering job information. This will be used to send off jobs to all other rendering farm
#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    /// Path to the output directory where final render image will be saved to
    pub output: PathBuf,
    /// Target node machines to run the job to
    pub nodes: Vec<RenderNode>,
    /// What kind of mode should this job run as
    pub mode: Mode,
    /// What version of blender we need to use to render this project job.
    pub version: Version,
    /// Path to blender files
    pub project_file: ProjectFile,
    /// Path to completed image result - May not be needed?
    pub renders: HashSet<RenderInfo>, // frame, then path to completed image source.
    #[serde(skip)]
    handlers: Vec<JoinHandle<JobStatus>>,
}

impl Job {
    pub fn new(
        project_file: &ProjectFile,
        output: &Path,
        version: &Version,
        nodes: Vec<RenderNode>,
        mode: Mode,
    ) -> Job {
        Job {
            nodes,
            output: output.to_path_buf().clone(),
            version: version.to_owned(),
            project_file: project_file.clone(),
            renders: HashSet::new(),
            mode,
            handlers: Vec::new(),
        }
    }

    // this method should be treated as a manager style where this job will be responsible to
    // divided the job into smaller frame task, and distribute across different target nodes on the network
    pub fn execute(&mut self) -> Result<()> {
        // This is where we will implement handlers group to monitor and manage threaded task.
        // One thread to monitor one job nodes
        self.run(1);
        Ok(())

        // let handle = thread::spawn(|| JobStatus::Completed);
        // self.handlers.push(handle);
    }

    // TODO: consider about how I can invoke this command from network protocol?
    /// Invoke blender to run the job
    pub fn run(&mut self, frame: i32) -> Result<RenderInfo> {
        // TODO: How can I split this up to run async task? E.g. Keep this task running while we still have frames left over.
        let args = Args::new(
            self.project_file.src.clone(),
            self.output.clone(),
            Mode::Frame(frame),
        );

        // TOOD: How do I find a way when a job is completed, invoke what frame it should render next.
        // TODO: This looks like I could move this code block somewhere else?
        let mut server_settings = ServerSetting::load();
        let blender = server_settings.get_blender(self.version.clone());

        // here's the question - if I'm on a network node, how do I send the host the image of the completed rendered job?
        let path = PathBuf::from(blender.render(&args).unwrap());

        // Return completed render info to the caller
        let info = RenderInfo { frame, path };
        Ok(info)
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.project_file == other.project_file
    }
}
