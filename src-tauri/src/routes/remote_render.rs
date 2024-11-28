/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use crate::{services::network_service::UiMessage, AppState};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

/// List all of the available blender version.
// TODO: Check and see why React is Re-rendering the page twice?
#[command(async)]
pub async fn list_versions(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let manager = server.manager.read().unwrap();
    let mut versions: Vec<Version> = manager
        .home
        .as_ref()
        .iter()
        .map(|b| match b.fetch_latest() {
            Ok(download_link) => download_link.get_version().clone(),
            Err(_) => Version::new(b.major, b.minor, 0), // I'm not sure why I need this? This seems like a bad idea?
        })
        .collect();

    let manager = server.manager.read().unwrap();
    let mut installed: Vec<Version> = manager
        .get_blenders()
        .iter()
        .map(|b| b.get_version().clone())
        .collect();

    versions.append(&mut installed);

    // is this function working?
    Ok(serde_json::to_string(&versions).expect("Unable to serialize version list!"))
}

// TODO: Reclassify this function behaviour - Should it pop the node off the network? Should it send disconnect signal? Should it shutdown node remotely?
// Describe the desire behaviour for this implementation.
#[command]
pub fn delete_node(target_node: String) -> Result<(), Error> {
    dbg!(target_node);
    // delete node from list and refresh the app?
    // let node_mutex = &app.state::<Mutex<Data>>();
    // let mut node = node_mutex.lock().unwrap();
    // node.render_nodes.retain(|x| x != &target_node);
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct BlenderInfo {
    blend_version: Version,
    frame: i32,
    // could also provide other info like Eevee or Cycle?
}

#[command(async)]
pub async fn import_blend(
    state: State<'_, Mutex<AppState>>,
    path: PathBuf,
) -> Result<String, String> {
    // open dialog here
    // let assume that we received a path back from the dialog                                                                  // then if we have a valid file - use .blend from blender to peek into the file.
    // blend::

    // let server = state.lock().unwrap();
    // let manager = server.manager.read().unwrap();

    // let data = manager.peek(path);
    // let blend = match blend::Blend::from_path(&path) {
    //     Ok(obj) => obj,
    //     Err(_) => return Err("Fail to load blender file!".to_owned()),
    // };

    // for obj in blend.root_instances() {
    //     dbg!(obj);
    // }

    // wanted to see if this mini closure will help free data block>
    // Using this closer as a test experimental code for now.
    {
        // take a look into kad?
        // Right here, using kad, we will send out a new provider to publish a file available to download from the network.
        // client will request the file when it receives a job from this host.

        // let data = std::fs::read(&path).unwrap();
        let file_name = path
            .file_name()
            .expect("Should be a valid file from above")
            .to_str()
            .unwrap()
            .to_owned();
        let app_state = state.lock().await;
        if let Err(e) = app_state
            .to_network
            .send(UiMessage::Status(file_name))
            .await
        {
            println!("Fail to send to network from application state {e:?}");
        }
    }

    // Here I'd like to know how I can extract information from the blend file, such as version number, Eevee/Cycle usage, Frame start and End. For now get this, and then we'll expand later
    let info = BlenderInfo {
        blend_version: Version::new(4, 1, 0),
        frame: 1,
    };

    let data = serde_json::to_string(&info).unwrap();
    Ok(data)
}

// Wonder why this was commented out?
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
