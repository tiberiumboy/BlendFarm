/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use crate::{
    models::project_file::ProjectFile,
    // services::server::Server,
    AppState,
};
use blender::models::home::BlenderHome;
use semver::Version;
use std::sync::Mutex;
use tauri::{command, AppHandle, Error, State};

/// Connect to a node
#[command]
pub fn create_node(
    // _state: State<Mutex<AppState>>,
    _host: String,
    _name: String,
    _port: u16,
) -> Result<String, Error> {
    todo!("read comments below");
    /*
       This function seems a bit confusing. As a server, we shouldn't be able to connect, instead we broadcast our public address if we receive a ping from the client.
       If a client joins the network, that client send out advertisement packet to inform the server client availability on the network.
       Once client receives the server's ping back, the client will try to connect to the server using the IP address provided
       For this method - this seems a bit counterintuitive. The idea behind this method is as follows:
       Server start up, tries connect to client via IP address
       This method impl. would make for certain situation. (E.g. UDP is blocked/need manual way of adding NetworkNode via CLI)
       However, this doesn't fit in the current milestone goal of making this application work.
    */
    // Got an invalid socket address syntax from this line?
    // let socket: SocketAddr = host.parse().unwrap();

    // let server = state.lock().unwrap();
    // server.connect(name, socket);

    // println!("parsed socket successfully! {:?}", &socket);
    // Ok("Node created successfully".to_string())
}

/// List out all available node for this blendfarm.
#[command]
pub fn list_node(_state: State<Mutex<AppState>>)
// -> Result<String, Error>
{
    // let _server = state.lock().unwrap();
    // server.get_peer_list(); // hmm might be a problem here?
    // let data = serde_json::to_string(col.render_nodes).unwrap();
    // Ok(data)
}

#[command]
pub fn ping_node(_state: State<Mutex<AppState>>) -> Result<String, Error> {
    // let _server = state.lock().unwrap();
    // server.ping();
    Ok("Ping sent!".to_string())
}

/// List all of the available blender version.
#[command(async)]
pub fn list_versions(state: State<Mutex<AppState>>) -> Result<String, String> {
    // I'd like to know why this function was invoked twice?
    let server = state.lock().unwrap();
    if let Ok(blender_link) = server.manager.read() {
        let versions: Vec<Version> = blender_link
            .list_all_blender_version()
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

// TODO: Reclassify this function behaviour - Should it pop the node off the network? Should it send disconnect signal? Should it shutdown node remotely?
// Describe the desire behaviour for this implementation.
#[command]
pub fn delete_node(_app: AppHandle, target_node: String) -> Result<(), Error> {
    dbg!(target_node);
    // delete node from list and refresh the app?
    // let node_mutex = &app.state::<Mutex<Data>>();
    // let mut node = node_mutex.lock().unwrap();
    // node.render_nodes.retain(|x| x != &target_node);
    Ok(())
}

/// Delete target project file from the collection. Note - this does not mean delete the original source file, it simply remove the project entry from the list
#[command]
pub fn delete_project(_project_file: ProjectFile) -> Result<(), String> {
    // TODO: The idea behind this method is to clear the visual entry on the app, not to modify the original blender file.
    // In a realistic scenario, we should never delete blend file, unless they were transfer over the network and stored temporarily (In this case, those file should be stored in a tmp location on the client node side...)
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
