// this is the settings controller section that will handle input from the setting page.
use crate::models::server_setting::ServerSetting;
// use blender::blender::Blender;
use blender::{blender::Blender, manager::Manager as BlenderManager};
use std::path::PathBuf;
use tauri::{command, Error};

/*
    Developer Blog
    I'm slowly breaking apart serversettings because the name itself does not imply settings configuration for _all_ other services in this application.
    TODO: Create ClientSettings, and create a BlendFarmConfiguration file to hold all settings and configurations
*/

/// List out currently saved blender installation on the machine
#[command]
pub fn list_blender_installation() -> Result<String, Error> {
    let manager = BlenderManager::load();
    let blenders = manager.get_blenders();
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
    // TODO - Move this code implementation to Blender implementation instead.
    let extension = Blender::get_extension().unwrap();
    let str_path = path.as_os_str().to_str().unwrap().to_owned();

    let blender = if str_path.contains(&extension) {
        let folder_name = &path
            .file_name()
            .unwrap()
            .to_os_string()
            .to_str()
            .unwrap()
            .replace(&extension, "");

        // this feels wrong, should this be called from Blender instead of BlenderDownloadLink? I want to treat Blender as our only source of public API access to.
        Blender::from_content(&path, folder_name).unwrap()
    } else {
        // for MacOS - for some unknown reason, user cannot navigate into the app bundle, therefore we must include the path ourselves here.
        if let "macos" = std::env::consts::OS {
            path = path.join("Contents/MacOS/Blender")
        }
        Blender::from_executable(path).unwrap()
    };

    // Add to the server settings
    let mut manager = BlenderManager::load();
    manager.add(&blender);
    Ok(())
}

#[command(async)]
pub fn remove_blender_installation(blender: Blender) -> Result<(), Error> {
    let mut manager = BlenderManager::load();
    manager.remove(&blender);
    Ok(())
}
