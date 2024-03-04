use crate::models::project_file::ProjectFile;
use crate::models::render_node::RenderNode;

pub struct Context {
    pub id: Option<String>,
    pub render_nodes: Vec<RenderNode>,
    pub project_files: Vec<ProjectFile>,
}
