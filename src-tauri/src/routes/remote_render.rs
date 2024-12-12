/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use crate::AppState;
use blender::blender::Blender;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, State};
use tokio::sync::Mutex;

/// List all of the available blender version.
#[command(async)]
pub async fn list_versions(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let manager = server.manager.read().await;
    let mut versions: Vec<Version> = manager
        .home
        .as_ref()
        .iter()
        .map(|b| match b.fetch_latest() {
            Ok(download_link) => download_link.get_version().clone(),
            Err(_) => Version::new(b.major, b.minor, 0), // I'm not sure why I need this? This seems like a bad idea?
        })
        .collect();

    let manager = server.manager.read().await;
    let mut installed: Vec<Version> = manager
        .get_blenders()
        .iter()
        .map(|b| b.get_version().clone())
        .collect();

    versions.append(&mut installed);

    // is this function working?
    Ok(serde_json::to_string(&versions).expect("Unable to serialize version list!"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderInfo {
    file_name: String,
    path: PathBuf,
    blend_version: Version,
    frame: i32,
    // could also provide other info like Eevee or Cycle?
}

// TODO: First time loading project would not fetch the file content properly? Why?
#[command(async)]
pub async fn import_blend(path: PathBuf) -> Result<String, String> {
    // TODO: Is there any differences using file dialog from Javascript side or rust side?
    let file_name = match path.file_name() {
        Some(str) => str.to_str().unwrap().to_owned(),
        None => return Err("Should be a valid file!".to_owned())
    };

    let data = match Blender::peek(&path).await {
        Ok(data) => data,
        Err(e) => return Err(e.to_string()),
    };

    let info = BlenderInfo {
        file_name,
        path,
        blend_version: data.last_version,
        frame: data.frame_start,
    };

    let data = serde_json::to_string(&info).unwrap();
    Ok(data)
}
