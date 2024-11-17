// this is the settings controller section that will handle input from the setting page.
use crate::models::server_setting::ServerSetting;
use blender::{blender::Blender, manager::Manager as BlenderManager};
use semver::Version;
use std::path::PathBuf;
use tauri::{command, Error};

/*
Developer Blog
- Ran into an issue trying to unpack blender on MacOS - turns out that .ends_with needs to match the child as a whole instead of substring.
- Changed the code down below to rely on using AppState, which contains managers needed to access to or modify to.
TODO: Newly added blender doesn't get saved automatically.
*/

/// List out currently saved blender installation on the machine
#[command]
pub fn list_blender_installation() -> Result<String, Error> {
    let manager = BlenderManager::load();
    let blenders = manager.get_blenders();
    let data = serde_json::to_string(&blenders).unwrap();
    Ok(data)
}

#[command(async)]
pub fn get_server_settings() -> Result<ServerSetting, Error> {
    let server_settings = ServerSetting::load();
    Ok(server_settings)
}

#[command]
pub fn set_server_settings(
    new_settings: ServerSetting,
) -> Result<(), String> {
    // maybe I'm a bit confused here?
    new_settings.save();
    Ok(())
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub fn add_blender_installation(
    path: PathBuf,
) -> Result<Blender, Error> {
    let mut manager = BlenderManager::load();
    match manager.add_blender_path(&path) {
        Ok(blender) => Ok(blender),
        Err(e) => Err(Error::AssetNotFound(e.to_string())),
    }
}

#[command(async)]
pub fn fetch_blender_installation(
    version: &str,
) -> Result<Blender, String> {
    let mut manager = BlenderManager::load();
    let version = Version::parse(version).map_err(|e| e.to_string())?;
    let blender = manager.fetch_blender(&version).map_err(|e| match e {
        blender::manager::ManagerError::DownloadNotFound { arch, os, url } => {
            format!("Download link not found! {arch} {os} {url}")
        }
        blender::manager::ManagerError::RequestError(request) => {
            format!("Request error: {request}")
        }
        blender::manager::ManagerError::FetchError(fetch) => format!("Fetch error: {fetch}"),
        blender::manager::ManagerError::IoError(io) => format!("IoError: {io}"),
        blender::manager::ManagerError::UnsupportedOS(os) => format!("Unsupported OS {os}"),
        blender::manager::ManagerError::UnsupportedArch(arch) => {
            format!("Unsupported architecture! {arch}")
        }
        blender::manager::ManagerError::UnableToExtract(ctx) => {
            format!("Unable to extract content! {ctx}")
        }
        blender::manager::ManagerError::UrlParseError(url) => format!("Url parse error: {url}"),
        blender::manager::ManagerError::PageCacheError(cache) => {
            format!("Page cache error! {cache}")
        }
        blender::manager::ManagerError::BlenderError { source } => {
            format!("Blender error: {source}")
        }
    })?;
    Ok(blender)
}

// TODO: Ambiguous name - Change this so that we have two methods,
// - Severe local path to blender from registry (Orphan on disk/not touched)
// - Delete blender content completely (erasing from disk)
#[command(async)]
pub fn remove_blender_installation(
    blender: Blender,
) -> Result<(), Error> {
    let mut manager = BlenderManager::load();
    manager.remove_blender(&blender);
    Ok(())
}
