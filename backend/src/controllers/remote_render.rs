use crate::models::project_file::ProjectFile;
use crate::models::{data::Data, job::Job, render_node::RenderNode};
use blender::mode::Mode;
// use blender::page_cache::PageCache;
use semver::Version;
use std::{path::PathBuf, sync::Mutex /* thread */};
use tauri::{command, Manager};
use tauri::{AppHandle, Error};

// We'll use this to organize and collect render nodes information
// pub struct Context {
//     pub nodes: Mutex<Vec<RenderNode>>,
//     pub projects: Mutex<Vec<ProjectFile>>,
//     pub jobs: Mutex<Vec<Job>>,
// }

/// Create a node
#[command]
pub fn create_node(_app: AppHandle, name: &str, host: &str) -> Result<String, Error> {
    // Got an invalid socket address syntax from this line?
    let socket = host.parse().expect("Unable to parse socket address!");
    let mut node = RenderNode::new(name, socket);

    println!("created our node successfully! {:?}", &node);
    // hmm would this become a problem?
    let _ = node.listen();

    println!("Successfully listened!");

    // let data = serde_json::to_string(&node).unwrap();
    // match node.connect() {
    //     Ok(node) => {
    // let node_mutex = app.state::<Mutex<Data>>();
    // let mut col = node_mutex.lock().unwrap();
    //         col.render_nodes.push(node);
    Ok("Node created successfully".to_string())
    //     }
    //     Err(e) => {
    //         println!("Error: {}", e);
    //         Err(Error::from(e))
    //     }
    // }
}

/// List out all available node for this blendfarm.
#[command]
pub fn list_node(app: AppHandle) -> Result<String, Error> {
    let node_mutex = app.state::<Mutex<Data>>();
    let col = node_mutex.lock().unwrap();
    let data = serde_json::to_string(&col.render_nodes).unwrap();
    Ok(data)
}

/// List all of the available blender version. (TODO - Impl. list of blender version available to download?)
#[command]
pub fn list_versions() -> Result<String, Error> {
    // let cache = PageCache::load();
    // TODO: Find a way to load blender version here? See how Blendfarm does it?
    let versions = vec![Version::new(3, 0, 1), Version::new(4, 1, 0)];
    // let contents = cache.fetch(url);
    let contents = serde_json::to_string(&versions).unwrap();
    Ok(contents)
}

/// Edit target node and update config with the new configuration set for the target node
#[command]
pub fn edit_node(_app: AppHandle, _update_node: RenderNode) {
    todo!();
}

/// Delete target node from the configuration
#[command]
pub fn delete_node(app: AppHandle, target_node: RenderNode) -> Result<(), Error> {
    // delete node from list and refresh the app?
    let node_mutex = &app.state::<Mutex<Data>>();
    let mut node = node_mutex.lock().unwrap();
    node.render_nodes.retain(|x| x != &target_node);
    Ok(())
}

/// Allow user to import a new project file for blenderfarm to process job for.
#[command]
pub fn import_project(app: AppHandle, path: &str) {
    let file_path = PathBuf::from(path);
    let mut project_file = ProjectFile::new(file_path);

    let ctx_mutex = app.state::<Mutex<Data>>();
    let mut ctx = ctx_mutex.lock().unwrap();
    project_file.move_to_temp();
    ctx.project_files.push(project_file);
}

/// Delete target project file from the collection. Note - this does not mean delete the original source file, it simply remove the project entry from the list
#[command]
pub fn delete_project(app: AppHandle, project_file: ProjectFile) {
    // retain the project from the collection.
    let ctx = app.state::<Mutex<Data>>();
    let mut data = ctx.lock().unwrap();
    let mut project = data.get_project_file(&project_file).unwrap().to_owned();
    project.clear_temp();
    data.project_files.retain(|x| *x != project_file);
}

#[command]
pub fn list_projects(app: AppHandle) -> Result<String, Error> {
    let ctx_mutex = app.state::<Mutex<Data>>();
    let ctx = ctx_mutex.lock().unwrap();
    let data = serde_json::to_string(&ctx.project_files).unwrap();
    Ok(data)
}

/// Create a new job from the list of project collection, and begin network rendering from target nodes.
#[command(async)]
pub fn create_job(
    output: &str,
    version: &str,
    project_file: ProjectFile,
    // may no longer need this?
    // nodes: Vec<RenderNode>,
    mode: Mode,
) {
    let output: PathBuf = PathBuf::from(output);
    let version = Version::parse(version).unwrap();
    let mut _job = Job::new(project_file, output, version, mode);
    // fetch me the server, and push new job to the server.
}

/// Abort the job if it's running and delete the entry from the collection list.
#[command]
pub fn delete_job(app: AppHandle, target_job: Job) {
    let ctx = app.state::<Mutex<Data>>();
    let mut data = ctx.lock().unwrap();
    // TODO: before I do this, I need to go through each of the nodes and stop this job.
    data.jobs.retain(|x| x != &target_job);
}

/// List all available jobs stored in the collection.
#[command]
pub fn list_job(app: AppHandle) -> Result<String, Error> {
    let job_mutex = app.state::<Mutex<Data>>();
    let job = job_mutex.lock().unwrap();
    let data = serde_json::to_string(&job.jobs).unwrap();
    Ok(data)
}
