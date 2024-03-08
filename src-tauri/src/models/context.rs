use crate::models::project_file::ProjectFile;
use crate::models::render_node::RenderNode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Context {
    pub render_nodes: Vec<RenderNode>,
    pub project_files: Vec<ProjectFile>,
}
