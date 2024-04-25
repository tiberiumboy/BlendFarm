use crate::models::project_file::ProjectFile;
use crate::models::{data::Data, job::Job, render_node::RenderNode};
use std::{path::PathBuf, sync::Mutex};
use tauri::api::dialog::FileDialogBuilder;
use tauri::Error;
use tauri::{command, Manager};

// soon I want to return the client node it established to
#[command]
pub fn create_node(app: tauri::AppHandle, name: &str, host: &str) -> Result<String, Error> {
    let node = RenderNode::parse(name, host).unwrap();
    let node_mutex = app.state::<Mutex<Data>>();
    let mut col = node_mutex.lock().unwrap();
    let data = serde_json::to_string(&node).unwrap();
    let node = node.connect();
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

// TODO: Change this to handle string input of files for new project file.
#[tauri::command]
pub fn create_job(app: tauri::AppHandle, path: &str) {
    // app crashed when api block thread. Do not use tauri::api::dialog::blocking::* apis.
    // Problem here - I need to find a way to ask the user about the blender file,
    // how do I know which blender version should I use to render?
    // How can I block js from invoking next when I need to wait for this dialog to complete?
    let ctx_mutex = app.state::<Mutex<Data>>();
    let mut ctx = ctx_mutex.lock().unwrap();
    let file_path = PathBuf::from(path);
    let project_file = ProjectFile::new(&file_path);
    let job = Job::new(project_file);

    //for node in ctx.render_nodes.iter() {
    // send the job to the node then invoke to run it?
    // node.send(job.project_file.file_path());
    //}

    ctx.jobs.push(job);
    // can we have some sort of mechanism to hold data collection as long as this program is alive?
    // something we can append this list to the collections and reveal?
}

#[tauri::command]
pub fn edit_job(app: tauri::AppHandle, update_job: Job) {
    let job_mutex = &app.state::<Mutex<Data>>();
    let mut job = job_mutex.lock().unwrap();
    job.jobs.retain(|x| x.id != update_job.id);
    job.jobs.push(update_job); // I see a problem here, we're pushing modified job at the bottom of the list, Wish we could just update at position?
}

#[allow(dead_code)]
#[tauri::command]
pub fn delete_job(app: tauri::AppHandle, id: String) {
    let job_mutex = app.state::<Mutex<Data>>();
    let mut job = job_mutex.lock().unwrap();
    job.jobs.retain(|x| x.id != id); // TODO: See if there's a deconstructor that I need to be consern about.
}

#[allow(dead_code)]
#[tauri::command]
pub fn list_job(app: tauri::AppHandle) -> Result<String, Error> {
    let job_mutex = app.state::<Mutex<Data>>();
    let job = job_mutex.lock().unwrap();
    let data = serde_json::to_string(&job.jobs).unwrap();
    Ok(data)
}
