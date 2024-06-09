use std::{
    io::Result,
    path::{Path, PathBuf},
};

use super::{project_file::ProjectFile, render_node::RenderNode, server_setting::ServerSetting};
// use crate::services::sender;
use blender::{args::Args, blender::Blender, mode::Mode};
use semver::Version;
use serde::{Deserialize, Serialize};

// pub trait JobStatus {}
#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
// pub struct Paused;
// find a way to parse output data, and provide percentage of completion here
// pub struct Running(f32); // percentage of completion
// pub struct Completed;
// pub struct Error(String);

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
    #[allow(dead_code)]
    pub fn run(&self) -> Result<String> {
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
