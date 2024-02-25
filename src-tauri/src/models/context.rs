use crate::{page::project::ProjectFile, render_client::RenderClient};

pub struct Context {
    pub id: Option<String>,
    pub render_nodes: Vec<RenderClient>,
    pub project_files: Vec<ProjectFile>,
}
