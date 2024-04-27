use crate::models::project_file::ProjectFile;
use crate::models::{data::Data, job::Job, render_node::RenderNode};
use std::{path::PathBuf, sync::Mutex};
use tauri::{command, Manager};
use tauri::{AppHandle, Error};

// soon I want to return the client node it established to
#[command]
pub fn create_node(app: AppHandle, name: &str, host: &str) -> Result<String, Error> {
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
pub fn list_node(app: AppHandle) -> Result<String, Error> {
    let node_mutex = app.state::<Mutex<Data>>();
    let col = node_mutex.lock().unwrap();
    let data = serde_json::to_string(&col.render_nodes).unwrap();
    Ok(data)
}

#[command]
pub fn edit_node(_app: AppHandle, _update_node: RenderNode) {}

#[command]
pub fn delete_node(app: AppHandle, id: String) -> Result<(), Error> {
    // delete node from list and refresh the app?
    let node_mutex = &app.state::<Mutex<Data>>();
    let mut node = node_mutex.lock().unwrap();
    node.render_nodes.retain(|x| x.id != id);
    Ok(())
}

// TODO: Change this to handle string input of files for new project file.
#[command]
pub fn import_project(app: AppHandle, path: &str) {
    let file_path = PathBuf::from(path);
    let mut project_file = ProjectFile::new(&file_path);

    let ctx_mutex = app.state::<Mutex<Data>>();
    let mut ctx = ctx_mutex.lock().unwrap();
    project_file.move_to_temp();
    ctx.project_files.push(project_file);
}

#[command]
pub fn sync_project(app: AppHandle, id: &str) {
    // we find the project by the id, then we re-sync the files
    let ctx = app.state::<Mutex<Data>>();
    let mut data = ctx.lock().unwrap();
    let project = data.project_files.iter_mut().find(|x| x.id == id).unwrap();
    project.move_to_temp();
}

#[command]
pub fn delete_project(app: AppHandle, id: &str) {
    // retain the project from the collection.
    let ctx = app.state::<Mutex<Data>>();
    let mut data = ctx.lock().unwrap();
    // TODO: Find a way to clear BlenderFiles if someone decided to delete the project file.
    // let mut project = data.project_files.iter().find(|x| x.id == id).unwrap();
    // project.clear_temp();
    data.project_files.retain(|x| x.id != id);
}

#[command]
pub fn list_projects(app: AppHandle) -> Result<String, Error> {
    let ctx_mutex = app.state::<Mutex<Data>>();
    let ctx = ctx_mutex.lock().unwrap();
    let data = serde_json::to_string(&ctx.project_files).unwrap();
    Ok(data)
}

#[command]
pub fn create_job(app: AppHandle, output: &str, project_id: &str, nodes: Vec<RenderNode>) {
    let ctx = app.state::<Mutex<Data>>();
    let mut data = ctx.lock().unwrap();
    let project = data
        .project_files
        .iter()
        .find(|x| x.id == project_id)
        .unwrap();
    let output = PathBuf::from(output);
    let job = Job::new(&project.to_owned(), &output, nodes);

    data.jobs.push(job);
    job.run();
    // Ok cool now that we have a job up and running, we should send notification to start it?
}

#[command]
pub fn delete_job(app: AppHandle, id: String) {
    let ctx = app.state::<Mutex<Data>>();
    let mut data = ctx.lock().unwrap();
    // TODO: before I do this, I need to go through each of the nodes and stop this job.
    data.jobs.retain(|x| x.id != id); // TODO: See if there's a deconstructor that I need to be consern about.
}

#[command]
pub fn list_job(app: AppHandle) -> Result<String, Error> {
    let job_mutex = app.state::<Mutex<Data>>();
    let job = job_mutex.lock().unwrap();
    let data = serde_json::to_string(&job.jobs).unwrap();
    Ok(data)
}
