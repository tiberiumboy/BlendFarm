/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use super::{project_file::ProjectFile, render_info::RenderInfo};
use blender::blender::Manager;
use blender::models::{args::Args, mode::Mode, status::Status};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
    collections::HashSet,
    hash::Hash,
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};
use thiserror::Error;
use uuid::Uuid;

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

// how do I make this job extend it's lifespan? I need to monitor and regulate all on-going job method?
// if a node joins the server, we automatically assign a new active job to the node.
/// A container to hold rendering job information. This will be used to send off jobs to all other rendering farm
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    /// Unique job identifier
    id: Uuid,
    /// Path to the output directory where final render image will be saved to
    output: PathBuf,
    /// What kind of mode should this job run as
    mode: Mode,
    /// What version of blender we need to use to render this project job.
    version: Version,
    /// Path to blender files
    project_file: ProjectFile,
    // Path to completed image result - May not be needed?
    // how do I make hash ignore this?
    renders: HashSet<RenderInfo>,
    // I should probably take responsibility for this, Once render is complete - I need to send a signal back to the host saying here's the frame, and here's the raw image data.
    // This would be nice to have to have some kind of historical copy, but then again, all of this value is being sent to the server directly. we should not retain any data behind on the node to remain lightweight and easy on storage space.
    // pub renders: HashSet<RenderInfo>, // frame, then path to completed image source.
    current_frame: i32,
}

impl Job {
    pub fn new(project_file: ProjectFile, output: PathBuf, version: Version, mode: Mode) -> Job {
        let current_frame = match mode {
            Mode::Frame(frame) => frame,
            Mode::Animation { start, .. } => start,
            _ => 0,
        };
        Job {
            id: Uuid::new_v4(),
            version,
            output,
            project_file,
            mode,
            renders: Default::default(),
            current_frame,
        }
    }

    // TODO: consider about how I can invoke this command from network protocol?
    // Invoke blender to run the job
    // Find out if I need to run this locally, or just rely on the server to perform the operation?
    #[allow(dead_code)]
    pub fn run(&mut self, frame: i32) -> Result<RenderInfo> {
        let path: &Path = self.project_file.as_ref();
        // TODO: How can I split this up to run async task? E.g. Keep this task running while we still have frames left over.
        let args = Args::new(path, self.output.clone(), Mode::Frame(frame));

        // TOOD: How do I find a way when a job is completed, invoke what frame it should render next.
        // TODO: This looks like I could move this code block somewhere else?
        let mut manager = Manager::load();
        let blender = manager.fetch_blender(&self.version).unwrap();

        // here's the question - if I'm on a network node, how do I send the host the image of the completed rendered job?
        // yeah here's a good question?
        // we can use the same principle as we were doing before :o!! Nice?
        let listener = blender.render(args);

        while let Ok(status) = listener.recv() {
            // Return completed render info to the caller
            match status {
                Status::Completed { result } => {
                    let info = RenderInfo {
                        frame,
                        path: result,
                    };
                    return Ok(info);
                }
                Status::Error(e) => return Err(Error::new(ErrorKind::ConnectionAborted, e)),
                _ => {}
            }
        }

        Err(Error::new(
            ErrorKind::ConnectionRefused,
            "Unable to render!".to_owned(),
        ))
    }

    // TOOD: These commented out function appears best to be implemented in a manager class of some sort.
    /*
    fn compare_and_increment(&mut self, max: i32) -> Option<i32> {
        if self.current_frame < max {
            self.current_frame += 1;
            Some(self.current_frame)
        } else {
            None
        }
    }

    pub fn next_frame(&mut self) -> Option<i32> {
        match self.mode {
            Mode::Frame(frame) => self.compare_and_increment(frame),
            Mode::Animation { start: _, end } => self.compare_and_increment(end),
            _ => None,
        }
    }
    */
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
