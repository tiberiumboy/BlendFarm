// this is the settings controller section that will handle input from the setting page.
use crate::models::server_setting::ServerSetting;
use blender::blender::Blender;
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
    let mut server_settings = ServerSetting::load();
    let blender = Blender::from_executable(path).unwrap();
    server_settings.blenders.push(blender);
    server_settings.save();
    Ok(())
}
