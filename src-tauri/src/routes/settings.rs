// this is the settings controller section that will handle input from the setting page.
use crate::models::{app_state::AppState, server_setting::ServerSetting};
use blender::blender::Blender;
use maud::html;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

/*
Developer Blog
- Ran into an issue trying to unpack blender on MacOS - turns out that .ends_with needs to match the child as a whole instead of substring.
- Changed the code down below to rely on using AppState, which contains managers needed to access to or modify to.
TODO: Newly added blender doesn't get saved automatically.
*/

/// List out currently saved blender installation on the machine
#[command(async)]
pub async fn list_blender_installation(state: State<'_, Mutex<AppState>>) -> Result<String, Error> {
    let app_state = state.lock().await;
    let manager = app_state.manager.read().await;
    let blenders = manager.get_blenders();
    let data = serde_json::to_string(&blenders).unwrap();
    Ok(data)
}

#[derive(Serialize, Deserialize)]
pub struct SettingResponse {
    pub install_path: PathBuf,
    pub render_path: PathBuf,
    pub cache_path: PathBuf,
}

/*
    Because blender installation path is not store in server setting, it is infact store under blender manager,
    we will need to create a new custom response message to provide all of the information needed to display on screen properly
*/

#[command(async)]
pub async fn set_server_settings(
    state: State<'_, Mutex<AppState>>,
    new_settings: ServerSetting,
) -> Result<(), String> {
    // maybe I'm a bit confused here?
    let app_state = state.lock().await;
    let mut old_setting = app_state.setting.write().await;
    new_settings.save();
    *old_setting = new_settings;
    Ok(())
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub async fn add_blender_installation(
    state: State<'_, Mutex<AppState>>,
    path: PathBuf,
) -> Result<Blender, Error> {
    let app_state = state.lock().await;
    let mut manager = app_state.manager.write().await;
    match manager.add_blender_path(&path) {
        Ok(blender) => Ok(blender),
        Err(e) => Err(Error::AssetNotFound(e.to_string())),
    }
}

#[command(async)]
pub async fn fetch_blender_installation(
    state: State<'_, Mutex<AppState>>,
    version: &str,
) -> Result<Blender, String> {
    let app_state = state.lock().await;
    let mut manager = app_state.manager.write().await;
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
pub async fn remove_blender_installation(
    state: State<'_, Mutex<AppState>>,
    blender: Blender,
) -> Result<(), Error> {
    let app_state = state.lock().await;
    let mut manager = app_state.manager.write().await;
    manager.remove_blender(&blender);
    Ok(())
}

#[command(async)]
pub async fn setting_page(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().await;
    let server_settings = app_state.setting.read().await;
    let blender_manager = app_state.manager.read().await;

    let install_path = blender_manager.as_ref().to_owned();
    let cache_path = server_settings.blend_dir.clone();
    let render_path = server_settings.render_dir.clone();

    // draw and display the setting page here
    let content = html! {
        div class="content" {
            h1 { "Settings" };
            p { r"Here we list out all possible configuration this tool can offer to user.
                    Exposing rich and deep components to customize your workflow" };
            h3 { "Blender Installation Path:" };
            input value=(install_path.to_str().unwrap());
            h3 { "Blender File Cache Path:" };
            input value=(cache_path.to_str().unwrap());
            h3 { "Render cache directory:" };
            input value=(render_path.to_str().unwrap());
        };
    };

    Ok(content.into_string())
}
