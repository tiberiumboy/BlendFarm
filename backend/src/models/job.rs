use std::{
    env,
    io::Result,
    path::{Path, PathBuf},
};

use super::{project_file::ProjectFile, render_node::RenderNode};
// use crate::services::sender;
use blender::{args::Args, blender::Blender, mode::Mode};
use serde::{Deserialize, Serialize};

// pub trait JobStatus {}
#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
// pub struct Paused;
// find a way to parse output data, and provide percentage of completion here
// pub struct Running(f32); // percentage of completion
// pub struct Completed;
// pub struct Error(String);

// todo - how do we know what mode to use or what mode to run?
#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub output: PathBuf, // output path
    pub nodes: Vec<RenderNode>,
    pub mode: Mode,
    pub project_file: ProjectFile,
    pub image_pic: Option<String>,
}

impl Job {
    // TODO: Impl mode for job.
    pub fn new(
        project_file: &ProjectFile,
        output: &Path,
        nodes: Vec<RenderNode>,
        mode: Mode,
    ) -> Job {
        Job {
            nodes,
            output: output.to_path_buf().clone(),
            project_file: project_file.clone(),
            image_pic: None,
            mode,
        }
    }

    // find a way to deal with async future/task?
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
        // TODO: Find a way to get correct blender version before running job

        // TODO: Replace this to reference correct blender version.
        let path = match env::consts::OS {
            "linux" => PathBuf::from("/home/jordan/Downloads/blender/blender"),
            "macos" => PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
            _ => panic!("unsupported OS"),
        };

        let mut blender = Blender::from_executable(path).unwrap();
        blender.render(&args)
    }

    #[allow(dead_code)]
    pub fn pause(self) {
        todo!();
    }

    // cancel current job and provide error message "User abort the job."
    #[allow(dead_code)]
    pub fn abort(self, _msg: &str) {
        todo!();
    }

    #[allow(dead_code)]
    pub fn resume(self) {
        todo!();
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.project_file == other.project_file
    }
}
