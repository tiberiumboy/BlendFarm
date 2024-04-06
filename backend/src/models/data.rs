use crate::models::{render_node::RenderNode, server_setting::ServerSetting};
use serde::{Deserialize, Serialize};

use super::job::Job;

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Data {
    pub server_setting: ServerSetting,
    pub render_nodes: Vec<RenderNode>,
    pub jobs: Vec<Job>,
}
