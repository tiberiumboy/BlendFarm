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

#[command]
pub fn get_server_settings() -> Result<String, Error> {
    // TODO: Find out what I can do with this information here.
    let server_settings = ServerSetting::load();
    let data = serde_json::to_string(&server_settings).unwrap();
    Ok(data)
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub fn add_blender_installation(path: PathBuf) -> Result<(), Error> {
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

#[command(async)]
pub fn remove_blender_installation(blender: Blender) -> Result<(), Error> {
    let mut server_setting = ServerSetting::load();
    let mut collection = server_setting.blenders;
    if let Some(index) = &collection.iter().position(|element| element == &blender) {
        let _ = &collection.swap_remove(*index);
        server_setting.blenders = collection;
    }

    Ok(())
}
