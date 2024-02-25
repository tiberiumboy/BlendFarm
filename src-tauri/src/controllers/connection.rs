// using this as a stuct to hold information about connection.html

use crate::render_client::RenderClient;

// soon I want to return the client node it established to
#[tauri::command]
pub fn create_node(ip: String, port: u16) {
    // connect to the node based on configuration given
    // if succeed, create a new record and return a entry to the display view
    // then hold a reference to that object somehow for listener on network update
    // let client_node = ClientNode::new(ip, port);
    let _node = RenderClient::new(ip, port);
    // ctx.render_nodes.push(node);
}
