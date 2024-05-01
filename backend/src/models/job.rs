use std::{io::Result, marker::PhantomData, path::PathBuf};

use super::{project_file::ProjectFile, render_node::RenderNode};
use crate::services::sender;
use blender::{args::Args, blender::Blender, mode::Mode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait JobStatus {}
#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
pub struct Paused;
// find a way to parse output data, and provide percentage of completion here
pub struct Running(f32); // percentage of completion
pub struct Completed;
pub struct Error(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub output: PathBuf, // output path
    pub nodes: Vec<RenderNode>,
    pub project_file: ProjectFile,
}

impl Job {
    pub fn new(project_file: &ProjectFile, output: &PathBuf, nodes: Vec<RenderNode>) -> Job {
        Job {
            id: Uuid::new_v4().to_string(),
            nodes,
            output: output.clone(),
            project_file: project_file.clone(),
        }
    }

    // find a way to deal with async future/task?
    pub fn run(&self) {
        let args = Args::new(
            self.project_file.src.clone(),
            self.output.clone(),
            Mode::Frame(1),
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

        let path = PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender");
        let mut blender = Blender::from_executable(path).unwrap();
        blender.render(&args);
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
