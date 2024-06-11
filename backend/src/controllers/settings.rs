// this is the settings controller section that will handle input from the setting page.
use crate::models::server_setting::ServerSetting;
// use blender::blender::Blender;
use blender::blender::{Blender, BlenderDownloadLink};
use std::path::PathBuf;
use tauri::{command, Error};

/// List out currently saved blender installation on the machine
#[command]
pub fn list_blender_installation() -> Result<String, Error> {
    let server_settings = ServerSetting::load();
    let blenders = server_settings.blenders;
    let data = serde_json::to_string(&blenders).unwrap();
    Ok(data)
}

/// Add a new blender entry to the system, but validate it first!
#[command]
pub fn add_blender_installation(path: PathBuf) -> Result<(), Error> {
    // first thing first, check and see if we're interfacing with either the compressed version of blender or the actual executable app of blender.
    // Once we figure out if it's the compress -> Unpack them into our blenderData directory
    // If it's actual executable, reference the path directly instead.

    // What's the rust recommendation way of doing this?
    // I wanted to make sure that the user isn't just loading compressed file containing blender
    // and at the same time, I also wanted to make sure that whatever Operating system blender is reference, needs to associate with the path directly

    // TODO: finish this implementation when you can!
    let path: PathBuf = path;
    // if &path
    //     .as_os_str()
    //     .as_ref()
    //     .to_string()
    //     .contains([".zip", ".tar.xz", ".dmg"])
    // {
    //     // more likely this is a macos path.
    //     // we would need to defer the current path and assign the correct path to blender location.
    //     path = path.join("Contents/MacOS/Blender");
    // }
    let folder_name = "";
    let link = match BlenderDownloadLink::extract_content(&path, folder_name) {
        Ok(link) => link,
        Err(_) => panic!("Shouldn't happen?"),
    };
    let blender = Blender::from_executable(link).unwrap();

    // let blender = Blender::from_executable(path).unwrap();
    // let mut server_settings = ServerSetting::load();
    // server_settings.blenders.push(blender);
    // server_settings.save();
    Ok(())
}
