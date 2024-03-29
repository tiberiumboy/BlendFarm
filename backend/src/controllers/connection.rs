use crate::models::{data::Data, render_node::RenderNode};
use std::sync::Mutex;
use tauri::Error;
use tauri::{command, Manager};

// someday I would like to be able to pass on the argument for this. Should it load handler on the fly or allow it to be defined at compile time.
// pub fn connection() -> FnOnce<T> {
//     [create_node, list_node, edit_node, delete_node]
// }

// soon I want to return the client node it established to
#[command]
pub fn create_node(app: tauri::AppHandle, name: &str, host: &str) -> Result<String, Error> {
    let node = RenderNode::parse(name, host).unwrap();
    let node_mutex = app.state::<Mutex<Data>>();
    let mut col = node_mutex.lock().unwrap();
    let data = serde_json::to_string(&node).unwrap();
    let _ = &node.connect().unwrap();
    col.render_nodes.push(node);
    Ok(data)
}

#[command] // could be dangerous if we have exact function name on front end?
           // which direction are we calling the function from? The front or the end?
pub fn list_node(app: tauri::AppHandle) -> Result<String, Error> {
    let node_mutex = app.state::<Mutex<Data>>();
    let col = node_mutex.lock().unwrap();
    let data = serde_json::to_string(&col.render_nodes).unwrap();
    Ok(data)
}

#[command]
pub fn edit_node(_app: tauri::AppHandle, _update_node: RenderNode) {}

#[command]
pub fn delete_node(app: tauri::AppHandle, id: String) -> Result<(), Error> {
    // delete node from list and refresh the app?
    let node_mutex = &app.state::<Mutex<Data>>();
    let mut node = node_mutex.lock().unwrap();
    node.render_nodes.retain(|x| x.id != id);
    Ok(())
}
