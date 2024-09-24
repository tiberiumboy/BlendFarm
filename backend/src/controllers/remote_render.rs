use crate::models::{
    data::Data, job::Job, project_file::ProjectFile, render_node::RenderNode, server::Server,
    server_setting::ServerSetting,
};
use blender::blender::Manager;
use blender::models::mode::Mode;
use semver::Version;
use std::{ffi::OsStr, fs, net::SocketAddr, path::PathBuf, sync::Mutex};
use tauri::{command, AppHandle, Error, State};

/// Create a node
#[command]
pub fn create_node(state: State<Mutex<Server>>, name: &str, host: &str) -> Result<String, Error> {
    // Got an invalid socket address syntax from this line?
    let socket: SocketAddr = host.parse().unwrap();

    let server = state.lock().unwrap();
    server.connect(name, socket);

    println!("parsed socket successfully! {:?}", &socket);
    Ok("Node created successfully".to_string())
}

/// List out all available node for this blendfarm.
#[command]
pub fn list_node(state: State<Mutex<Server>>) /*-> Result<String, Error> */
{
    let server = state.lock().unwrap();
    server.get_peer_list(); // hmm might be a problem here?
                            // let data = serde_json::to_string(&col.render_nodes).unwrap();
                            // Ok(data)
}

#[command]
pub fn ping_node(state: State<Mutex<Server>>) -> Result<String, Error> {
    let server = state.lock().unwrap();
    server.ping();
    Ok("Ping sent!".to_string())
}

/// List all of the available blender version. (TODO - Impl. list of blender version available to download?)
#[command]
pub fn list_versions(state: State<Mutex<Manager>>) -> Result<String, Error> {
    // let cache = PageCache::load();
    // TODO: Find a way to load all existing blender version here? See how Blendfarm does it?
    let manager = state.lock().unwrap();
    let versions: Vec<Version> = manager
        .get_blenders()
        .iter()
        .map(|b| b.get_version().clone())
        .collect();
    // let contents = cache.fetch(url);
    let contents = serde_json::to_string(&versions).unwrap();
    Ok(contents)
}

/// Edit target node and update config with the new configuration set for the target node
#[command]
pub fn edit_node(_app: AppHandle, _update_node: RenderNode) {
    todo!("Not yet implemented!");
}

/// Delete target node from the configuration
#[command]
pub fn delete_node(_app: AppHandle, target_node: String) -> Result<(), Error> {
    dbg!(target_node);
    // delete node from list and refresh the app?
    // let node_mutex = &app.state::<Mutex<Data>>();
    // let mut node = node_mutex.lock().unwrap();
    // node.render_nodes.retain(|x| x != &target_node);
    Ok(())
}

/// Allow user to import a new project file for blenderfarm to process job for.
#[command]
pub fn import_project(state: State<Mutex<Server>>, path: &str) {
    let file_path = PathBuf::from(path);
    let project_file = ProjectFile::new(file_path).unwrap();

    let server = state.lock().unwrap();
    server.send_file(&project_file.file_path());
}

/// Delete target project file from the collection. Note - this does not mean delete the original source file, it simply remove the project entry from the list
#[command]
pub fn delete_project(project_file: ProjectFile) -> Result<(), String> {
    if let Err(e) = fs::remove_file(project_file.file_path()) {
        println!("Error deleting project file from local system: {e}");
        return Err(format!("Unable to delete file!\n{}", e));
    };
    Ok(())
}

#[command]
pub fn list_projects(state: State<Mutex<ServerSetting>>) -> Result<String, Error> {
    let server = state.lock().unwrap();
    let project_files = match !server.blend_dir.exists() {
        true => vec![],
        false => {
            // validate and see if this doesn't break
            // need to find a way to filter reading dir to only *.blend extension.
            match server.blend_dir.read_dir() {
                Ok(entries) => {
                    // let mut col = Vec::with_capacity(entries.count());
                    let mut col = Vec::with_capacity(20); // temp fixes
                    for entry in entries {
                        if let Ok(dir_entity) = entry {
                            let file_path = dir_entity.path();
                            if file_path.is_file()
                                && file_path.extension().unwrap().eq(OsStr::new("blend"))
                            {
                                let project_file = ProjectFile::new(file_path).unwrap();
                                col.push(project_file);
                            }
                        }
                    }

                    col
                }
                Err(_) => Vec::new(),
            }
        }
    };

    let data = serde_json::to_string(&project_files).unwrap();
    Ok(data)
}

/// Create a new job from the list of project collection, and begin network rendering from target nodes.
#[command(async)]
pub fn create_job(
    state: State<Mutex<Server>>,
    output: &str,
    version: &str,
    project_file: ProjectFile,
    mode: Mode,
) {
    let output: PathBuf = PathBuf::from(output);
    let version = Version::parse(version).unwrap();
    let job = Job::new(project_file, output, version, mode);

    // TODO: Find a way to send the job to the clients and render the job.
    let server = state.lock().unwrap();
    server.send_job(job);
}

/// Abort the job if it's running and delete the entry from the collection list.
#[command]
pub fn delete_job(state: State<Mutex<Data>>, target_job: Job) {
    let mut data = state.lock().unwrap();
    // TODO: before I do this, I need to go through each of the nodes and stop this job.
    data.jobs.retain(|x| x != &target_job);
}

/// List all available jobs stored in the collection.
#[command]
pub fn list_job(state: State<Mutex<Data>>) -> Result<String, Error> {
    let job = state.lock().unwrap();
    let data = serde_json::to_string(&job.jobs).unwrap();
    Ok(data)
}
