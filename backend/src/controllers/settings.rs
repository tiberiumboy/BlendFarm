// this is the settings controller section that will handle input from the setting page.
use crate::models::server_setting::ServerSetting;
use blender::blender::{Blender, Manager};
use std::{path::PathBuf, sync::Mutex};
use tauri::{command, AppHandle, Error};

/*
    Developer Blog
    I'm slowly breaking apart serversettings because the name itself does not imply settings configuration for _all_ other services in this application.
    TODO: Create ClientSettings, and create a BlendFarmConfiguration file to hold all settings and configurations
*/

/// List out currently saved blender installation on the machine
#[command]
pub fn list_blender_installation() -> Result<String, Error> {
    let manager = Manager::load();
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

#[command]
pub fn set_server_settings(new_settings: ServerSetting) -> Result<(), String> {
    new_settings.save();
    Ok(())
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub fn add_blender_installation(app: AppHandle, path: PathBuf) -> Result<(), Error> {
    // I need information in string so I could use the contains operand, if there's a better way to write this without having to cast into string, would be ideal
    // TODO: Optimized so I could check the extension without casting to string (memory intensive operation)
    // Add to the server settings
    // consider using manager in a context instead?
    // let mutex = app.state::<Mutex<Manager>>();
    // let mut manager = mutex.lock().unwrap();
    // manager.add_blender_path(&path).unwrap();
    Ok(())
}

#[command(async)]
pub fn remove_blender_installation(blender: Blender) -> Result<(), Error> {
    let mut manager = Manager::load();
    manager.remove_blender(&blender);
    Ok(())
}
