/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use std::{
    io::Result,
    path::{Path, PathBuf},
};

use super::{project_file::ProjectFile, render_node::RenderNode, server_setting::ServerSetting};
// use crate::services::sender;
use blender::{args::Args, blender::Blender, mode::Mode};
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("Job failed to run: {0}")]
    FailedToRun(String),
}

// pub trait JobStatus {}
#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
pub struct Paused;
// find a way to parse output data, and provide percentage of completion here
/// percentage of completion
pub struct Running {
    frame: f32,
}
/// The job has been completed, path refers to the directory which contains all of the completed render image data.
pub struct Completed {
    path: Path,
}

/// Job reported an error that needs to be handle carefully.
pub struct Error(JobError);

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
    pub image_pic: Option<String>,
}

impl Job {
    // TODO: See if we need this?
    #[allow(dead_code)]
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
            image_pic: None,
            mode,
        }
    }

    // TODO: consider about how I can invoke this command from network protocol?
    /// Invoke blender to run the job
    pub fn run(&self) -> Result<String> {
        // TODO: How can I split this up to run async task? E.g. Keep this task running while we still have frames left over.
        let args = Args::new(
            self.project_file.src.clone(),
            self.output.clone(),
            self.mode.clone(),
        );

        // need to send network packet to node to notify running job
        // before sending notification out
        // sender.send();
        // for ele in self.nodes {
        // how can we get ahead of this and say that each node now needs to get the files
        // ele.send(&self.project_file.src);
        // ele.render
        // let blender = Blender::new();
        // blender.render(&self.project_file, &self.output);
        // }

        // TOOD: How do I find a way when a job is completed, invoke what frame it should render next.
        // TODO: This looks like I could move this code block somewhere else?
        let mut server_settings = ServerSetting::load();
        let blender = match server_settings
            .blenders
            .iter()
            .find(|&x| x.version == self.version)
        {
            Some(blender) => blender.to_owned(),
            None => {
                let blender =
                    Blender::download(self.version.clone(), &server_settings.blender_dir).unwrap();
                server_settings.blenders.push(blender.clone());
                server_settings.save();
                blender
            }
        };

        // TODO: Find a way to handle the errors here?
        let path = blender.render(&args).unwrap();
        Ok(path)
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.project_file == other.project_file
    }
}
