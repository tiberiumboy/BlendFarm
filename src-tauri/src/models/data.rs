use crate::models::project_file::ProjectFile;
use crate::models::{
    render_node::RenderNode,
    server_setting::ServerSetting,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Data {
    pub server_setting: ServerSetting,
    pub render_nodes: Vec<RenderNode>,
    pub project_files: Vec<ProjectFile>,
}
