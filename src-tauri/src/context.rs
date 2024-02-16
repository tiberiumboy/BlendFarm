use crate::render_client::RenderClient;

pub struct Context {
    pub id: Option<String>,
    pub render_nodes: Vec<RenderClient>,
}
