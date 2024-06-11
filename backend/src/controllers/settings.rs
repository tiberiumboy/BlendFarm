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
#[command(async)]
pub fn add_blender_installation(path: PathBuf) -> Result<(), Error> {
    // first thing first, check and see if we're interfacing with either the compressed version of blender or the actual executable app of blender.
    // Once we figure out if it's the compress -> Unpack them into our blenderData directory
    // If it's actual executable, reference the path directly instead.

    // What's the rust recommendation way of doing this?
    // I wanted to make sure that the user isn't just loading compressed file containing blender
    // and at the same time, I also wanted to make sure that whatever Operating system blender is reference, needs to associate with the path directly

    // TODO: finish this implementation when you can!
    let mut path: PathBuf = path;

    // I need information in string so I could use the contains operand, if there's a better way to write this without having to cast into string, would be ideal
    // TODO: Optimized so I could check the extension without casting to string (memory intensive operation)
    let extension = Blender::get_extension().unwrap();
    let str_path = path.as_os_str().to_str().unwrap().to_owned();

    let executable_path = if str_path.contains(&extension) {
        let folder_name = &path
            .file_name()
            .unwrap()
            .to_os_string()
            .to_str()
            .unwrap()
            .replace(&extension, "");

        match BlenderDownloadLink::extract_content(&path, folder_name) {
            Ok(link) => link,
            Err(_) => panic!("Shouldn't happen?"),
        }
    } else {
        // if the user actually select the correct blender path, then we will try to run command to fetch version info
        // for MacOS - for some unknown reason, user cannot navigate into the app bundle, therefore we must include the path ourselves here.
        if let "macos" = std::env::consts::OS {
            path = path.join("Contents/MacOS/Blender")
        }

        path
    };

    let blender = Blender::from_executable(executable_path).unwrap();

    // Add to the server settings
    let mut server_settings = ServerSetting::load();
    server_settings.blenders.push(blender);
    server_settings.save();
    Ok(())
}
