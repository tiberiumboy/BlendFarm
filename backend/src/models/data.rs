use std::io::{ErrorKind, Result};

use crate::models::{
    project_file::ProjectFile, render_node::RenderNode, server_setting::ServerSetting,
};
use serde::{Deserialize, Serialize};

use super::job::Job;

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Data {
    pub server_setting: ServerSetting, // this local host machine configuration
    pub render_nodes: Vec<RenderNode>, // available node on the network
    pub project_files: Vec<ProjectFile>, //Vec<ProjectFile>, // Project library
    pub jobs: Vec<Job>,                // Keeps track of current job process
}

impl Data {
    pub fn get_project_file(&self, project_file: &ProjectFile) -> Result<&ProjectFile> {
        self.project_files
            .iter()
            .find(|x| *x == project_file)
            .ok_or(ErrorKind::NotFound.into())
    }

    // pub fn get_render_node(&self, id: &str) -> Result<&RenderNode> {
    //     self.render_nodes
    //         .iter()
    //         .find(|x| x.id == id)
    //         .ok_or(ErrorKind::NotFound.into())
    // }
}
