// this is the settings controller section that will handle input from the setting page.
use crate::models::server_setting::ServerSetting;
use blender::blender::{Blender, Manager};
use semver::Version;
use std::{path::PathBuf, sync::Mutex};
use tauri::{command, Error, State};

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

#[command(async)]
pub fn get_server_settings(state: State<Mutex<ServerSetting>>) -> Result<ServerSetting, Error> {
    let server_settings = state.lock().unwrap().clone();
    Ok(server_settings)
}

#[command]
pub fn set_server_settings(new_settings: ServerSetting) -> Result<(), String> {
    new_settings.save();
    Ok(())
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub fn add_blender_installation(
    state: State<Mutex<Manager>>,
    path: PathBuf,
) -> Result<Blender, Error> {
    // I need information in string so I could use the contains operand, if there's a better way to write this without having to cast into string, would be ideal
    // TODO: Optimized so I could check the extension without casting to string (memory intensive operation)
    // Add to the server settings
    // consider using manager in a context instead?
    let mut manager = state.lock().unwrap();
    let blender = manager.add_blender_path(&path).unwrap();
    Ok(blender)
}

#[command(async)]
pub fn fetch_blender_installation(
    state: State<Mutex<Manager>>,
    version: &str,
) -> Result<Blender, Error> {
    let mut manager = state.lock().unwrap();
    let version = Version::parse(version).unwrap();
    let blender = manager.fetch_blender(&version).unwrap();
    Ok(blender)
}

// TODO: Ambiguous name - Change this so that we have two methods,
// - Severe local path to blender from registry (Orphan on disk/not touched)
// - Delete blender content completely (erasing from disk)
#[command(async)]
pub fn remove_blender_installation(
    state: State<Mutex<Manager>>,
    blender: Blender,
) -> Result<(), Error> {
    let mut manager = state.lock().unwrap();
    manager.remove_blender(&blender);
    Ok(())
}
