use crate::models::render_node::RenderNode;
use crate::models::render_node_collection::RenderNodeCollection;
use std::io::Error;
use std::sync::Mutex;
use tauri::{window, Manager, Window};

// soon I want to return the client node it established to
#[tauri::command]
pub fn create_node(app: tauri::AppHandle, ip: String, port: u16) -> Result<RenderNode, Error> {
    let node = RenderNode::new(ip, port);
    let node_mutex = app.state::<Mutex<RenderNodeCollection>>();
    let mut col = node_mutex.lock()?;
    col.push(node);
    Ok(node);
}

#[tauri::command]
pub fn list_node(app: tauri::AppHandle, window: Window) {
    let node_mutex = app.state::<Mutex<RenderNodeCollection>>();
    let mut col = node_mutex.lock()?;
    window.emit("list_node", col);
    // list out the node that is available on the network here
}

#[tauri::command]
pub fn edit_node(_app: tauri::AppHandle, _update_node: RenderNode) {}

#[tauri::command]
pub fn delete_node(_app: tauri::AppHandle, _id: String) {
    // delete node from list and refresh the app?
}
