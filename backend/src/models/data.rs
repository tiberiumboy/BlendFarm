use crate::models::{
    job::JobStatus, project_file::ProjectFile, render_node::RenderNode,
    server_setting::ServerSetting,
};
use serde::{Deserialize, Serialize};

use super::job::Job;

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Data {
    pub server_setting: ServerSetting, // this local host machine configuration
    pub render_nodes: Vec<RenderNode>, // available node on the network
    pub project_files: Vec<ProjectFile>, // Project library
    pub jobs: Vec<Job>,                // Keeps track of current job process
}
