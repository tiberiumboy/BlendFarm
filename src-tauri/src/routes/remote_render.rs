/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use crate::models::message::Command;
use crate::AppState;
use semver::Version;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::{fs::OpenOptions, io::Write, path::PathBuf};
use tauri::{command, State};
use tokio::sync::Mutex;

/// List all of the available blender version.
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

#[derive(Debug, Serialize, Deserialize)]
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
    // let assume that we received a path back from the dialog
    // then if we have a valid file - use .blend from blender to peek into the file.
    let file_name = path
        .file_name()
        .expect("Should be a valid file from above")
        .to_str()
        .unwrap()
        .to_owned();

    let blend = match blend::Blend::from_path(&path) {
        Ok(obj) => obj,
        Err(_) => return Err("Fail to load blender file!".to_owned()),
    };

    // blender version are display as three digits number, e.g. 404 is major: 4, minor: 4.
    // treat this as a u16 major = u16 / 100, minor = u16 % 100;
    let value: u64 = std::str::from_utf8(&blend.blend.header.version)
        .expect("Fail to parse version into utf8")
        .parse()
        .expect("Fail to parse string to value");

    let major = value / 100;
    let minor = value % 100;

    let app_state = state.lock().await;
    // using scope to drop manager usage.
    let blend_version = {
        let manager = app_state.manager.read().unwrap();

        // Get the latest patch from blender home
        match manager
            .home
            .as_ref()
            .iter()
            .find(|v| v.major.eq(&major) && v.minor.eq(&minor))
        {
            // TODO: Find a better way to handle this without using unwrap
            Some(v) => v.fetch_latest().unwrap().as_ref().clone(),
            // potentially could be a problem, if there's no internet connection, then we can't rely on zero patch?
            // For now this will do.
            None => Version::new(major.into(), minor.into(), 0),
        }
    };

    // TODO: Find out how I can get the start frame? Surely it's somewhere in the scene file?
    // or maybe different camera have different start frame to begin with?
    // I was able to find some useful information with b"SC" which holds scene file. - Take a look in Log1.txt file
    // let mut debug_file = OpenOptions::new()
    //     .write(true)
    //     .create(true)
    //     .open("log.txt")
    //     .unwrap();
    //
    // for obj in blend.instances_with_code(b"SC".to_owned()) {
    //     let name = obj.get("id").get_string("name");
    //     debug_file
    //         .write(format!("{name}: {obj:?}\n").as_bytes())
    //         .unwrap();
    //     // dbg!(obj);
    //     // would be nice to write this file out to text file somehow?
    // }
    // debug_file.flush().unwrap();

    // take a look into kad?
    // Right here, using kad, we will create a new entry for the host provider to publish a file available to download from the network.
    if let Err(e) = app_state.to_network.send(Command::Status(file_name)).await {
        println!("Fail to send to network from application state {e:?}");
    }

    // Here I'd like to know how I can extract information from the blend file such as Eevee/Cycle usage, Frame start and End.
    let info = BlenderInfo {
        blend_version,
        frame: 1,
    };

    println!("Blend info: {:?}", &info);

    let data = serde_json::to_string(&info).unwrap();
    Ok(data)
}
