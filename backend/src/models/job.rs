use std::{marker::PhantomData, path::PathBuf};

use super::{project_file::ProjectFile, render_node::RenderNode};
use crate::services::sender;
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
pub struct Job<JobStatus = Idle> {
    pub id: String,
    pub output: PathBuf, // output path
    pub nodes: Vec<RenderNode>,
    pub project_file: ProjectFile,
    pub state: PhantomData<JobStatus>,
}

impl Job<Idle> {
    pub fn new(project_file: &ProjectFile, output: &PathBuf, nodes: Vec<RenderNode>) -> Job {
        Job {
            id: Uuid::new_v4().to_string(),
            nodes,
            output: output.clone(),
            project_file: project_file.clone(),
            state: PhantomData::<Idle>,
        }
    }
}

impl Job<Running> {
    pub fn pause(self) -> Job<Paused> {
        Job {
            id: self.id,
            nodes: self.nodes,
            output: self.output,
            project_file: self.project_file,
            state: PhantomData::<Paused>,
        }
    }
}

impl Job<Paused> {
    pub fn resume(self) -> Job<Running> {
        // need to send network packet to node to notify resuming job before sending notification out

        // sender.send();
        Job {
            id: self.id,
            nodes: self.nodes,
            output: self.output,
            project_file: self.project_file,
            state: PhantomData::<Running>,
        }
    }
}
