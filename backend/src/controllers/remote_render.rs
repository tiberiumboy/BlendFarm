use crate::models::project_file::ProjectFile;
use crate::models::{data::Data, job::Job, render_node::RenderNode};
use std::sync::Mutex;
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

#[tauri::command]
pub fn create_job(app: tauri::AppHandle) {
    // app crashed when api block thread. Do not use tauri::api::dialog::blocking::* apis.
    // could we not access tauri api side from react for filedialogbuilder?
    // How can I block js from invoking next when I need to wait for this dialog to complete?
    // how can I only filter .blend extension format?
    FileDialogBuilder::new()
        .add_filter("Blender Files", &["blend"])
        .pick_files(move |path| match path {
            Some(file_paths) => {
                let ctx_mutex = app.state::<Mutex<Data>>();
                let mut ctx = ctx_mutex.lock().unwrap();
                for file_path in file_paths.iter() {
                    let project_file = ProjectFile::new(file_path);
                    let mut job = Job::new(project_file);

                    for node in ctx.render_nodes.iter() {
                        // send the job to the node then invoke to run it?
                        let _ = node.send(job.project_file.clone()).unwrap();
                    }

                    job.run();
                    ctx.jobs.push(job);
                }
            }
            None => {
                // do nothing
            }
        });
    // can we have some sort of mechanism to hold data collection as long as this program is alive?
    // something we can append this list to the collections and reveal?
}

#[tauri::command]
pub fn edit_job(app: tauri::AppHandle, _update_job: Job) {
    // let job_mutex = app.state::<Mutex<Data>>();
    // let mut job = job_mutex.lock().unwrap();
    // job.jobs.push(update_job);
}

#[tauri::command]
pub fn delete_job(app: tauri::AppHandle, id: String) {
    let job_mutex = app.state::<Mutex<Data>>();
    let mut job = job_mutex.lock().unwrap();
    job.jobs.retain(|x| x.id != id);
}

#[tauri::command]
pub fn list_job(app: tauri::AppHandle) -> Result<String, Error> {
    let job_mutex = app.state::<Mutex<Data>>();
    let job = job_mutex.lock().unwrap();
    let data = serde_json::to_string(&job.jobs).unwrap();
    Ok(data)
}
