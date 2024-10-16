/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/

use crate::models::{data::Data, job::Job, project_file::ProjectFile, server::Server};
use blender::models::{download_link::BlenderHome, mode::Mode};
use semver::Version;
use std::{net::SocketAddr, path::PathBuf, sync::Mutex};
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
pub fn list_node(state: State<Mutex<Server>>)
// -> Result<String, Error>
{
    let server = state.lock().unwrap();
    server.get_peer_list(); // hmm might be a problem here?
                            // let data = serde_json::to_string(col.render_nodes).unwrap();
                            // Ok(data)
}

#[command]
pub fn ping_node(state: State<Mutex<Server>>) -> Result<String, Error> {
    let server = state.lock().unwrap();
    server.ping();
    Ok("Ping sent!".to_string())
}

/// List all of the available blender version.
#[command(async)]
pub fn list_versions(state: State<Mutex<BlenderHome>>) -> Result<String, String> {
    // I'd like to know why this function was invoked twice?
    if let Ok(blender_link) = state.lock() {
        let versions: Vec<Version> = blender_link
            .list
            .iter()
            .map(|b| match b.fetch_latest() {
                Ok(download_link) => download_link.get_version().clone(),
                Err(_) => Version::new(b.major, b.minor, 0),
            })
            .collect();

        return Ok(serde_json::to_string(&versions).expect("Unable to serialize version list!"));
    }

    Err("Function already been called, please wait!".to_owned())
}

// TODO: May not be needed here?
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

#[command]
pub fn create_job(
    state: State<Mutex<Server>>,
    file_path: PathBuf,
    output: PathBuf,
    version: &str,
    mode: Mode,
) -> Result<Job, Error> {
    let version = Version::parse(version).unwrap_or_else(|_| Version::new(4, 2, 2));
    let project_file = ProjectFile::new(file_path)?;
    let job = Job::new(project_file, output, version, mode);

    // send job to server
    let server = state.lock().unwrap();
    server.send_job(job.clone());

    Ok(job)
}

/// Delete target project file from the collection. Note - this does not mean delete the original source file, it simply remove the project entry from the list
#[command]
pub fn delete_project(_project_file: ProjectFile) -> Result<(), String> {
    // Extremely dangerous! Lost one of my blend file from this!!
    // TODO: find a better approach to remove the entry from the list instead of permanently deleting the files.
    // if let Err(e) = fs::remove_file(project_file.file_path()) {
    //     println!("Error deleting project file from local system: {e}");
    //     return Err(format!("Unable to delete file!\n{}", e));
    // };
    Ok(())
}

// #[command]
// pub fn list_jobs(state: State<Mutex<Server>>) -> Result<String, Error> {
// let server = state.lock().unwrap();
// TODO reduce nested statement here?
// this was used to list out all blend files from server settings struct.
// let project_files = match !server.blend_dir.exists() {
//     true => vec![],
//     false => {
//         // validate and see if this doesn't break
//         // need to find a way to filter reading dir to only *.blend extension.
//         match server.blend_dir.read_dir() {
//             Ok(entries) => {
//                 // let mut col = Vec::with_capacity(entries.count());
//                 let mut col = Vec::with_capacity(20); // temp fixes
//                 for entry in entries {
//                     if let Ok(dir_entity) = entry {
//                         let file_path = dir_entity.path();
//                         if file_path.is_file()
//                             && file_path.extension().unwrap().eq(OsStr::new("blend"))
//                         {
//                             let project_file = ProjectFile::new(file_path).unwrap();
//                             col.push(project_file);
//                         }
//                     }
//                 }
//                 col
//             }
//             Err(_) => Vec::new(),
//         }
//     }
// };

//     let jobs = server.get_job_list();
//     let data = serde_json::to_string(&jobs).unwrap(); // Can't imagine this breaking - who knows?
//     Ok(data)
// }

/// Abort the job if it's running and delete the entry from the collection list.
#[command]
pub fn delete_job(state: State<Mutex<Data>>, target_job: Job) {
    let mut data = state.lock().unwrap();
    // TODO: before I do this, I need to go through each of the nodes and stop this job.
    data.jobs.retain(|x| x != &target_job);
}

/// List all available jobs stored in the collection.
#[command]
pub fn list_jobs(state: State<Mutex<Server>>) -> Result<String, Error> {
    let server = state.lock().unwrap();
    let data = server.get_job_list();
    let data = serde_json::to_string(&data).unwrap();
    Ok(data)
}
