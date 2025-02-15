use std::{path::PathBuf, sync::Arc};

// this is the settings controller section that will handle input from the setting page.
use crate::models::{app_state::AppState, server_setting::ServerSetting};
use blender::blender::Blender;
use maud::html;
use semver::Version;
use serde_json::json;
use tauri::{command, AppHandle, Error, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::{
    join,
    sync::{Mutex, RwLock},
};

const SETTING: &str= "settings";

/*
    Because blender installation path is not store in server setting, it is infact store under blender manager,
    we will need to create a new custom response message to provide all of the information needed to display on screen properly
*/

#[command(async)]
pub async fn list_blender_installed(state: State<'_, Mutex<AppState>>) -> Result<String, ()> {
    let app_state = state.lock().await;
    let manager = app_state.manager.read().await;
    let localblenders = manager.get_blenders();

    Ok(html! {
        @for blend in localblenders {
            tr {
                td {
                    (blend.get_version().to_string())
                };
                td {
                    (blend.get_executable().to_str().unwrap())
                };
            };
        };
    }
    .0)
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub async fn add_blender_installation(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>, // TODO: Need to change this to string, string?
) -> Result<String, ()> {
    // TODO: include behaviour to search for file that contains blender.
    // so here's where
    let path = match app.dialog().file().blocking_pick_file() {
        Some(file_path) => match file_path {
            FilePath::Path(path) => path,
            FilePath::Url(url) => url.to_file_path().unwrap(),
        },
        None => return Err(()),
    };

    let app_state = state.lock().await;
    let mut manager = app_state.manager.write().await;
    match manager.add_blender_path(&path) {
        Ok(_blender) => Ok(html! {
           // HX-trigger="newBlender"
        }
        .0),
        Err(_) => Err(()),
    }
}

// So this can no longer be a valid api call?
// TODO: Reconsider refactoring this so that it's not a public api call. Deprecate/remove asap
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
pub async fn update_settings(
    state: State<'_, Mutex<AppState>>,
    install_path: String,
    cache_path: String,
    render_path: String,
) -> Result<String, String> {
    let install_path = PathBuf::from(install_path);
    let blend_dir = PathBuf::from(cache_path);
    let render_dir = PathBuf::from(render_path);

    {
        let mut server = state.lock().await;
        server.setting = Arc::new(RwLock::new(ServerSetting {
            blend_dir,
            render_dir,
        }));
        let mut manager = server.manager.write().await;
        manager.set_install_path(&install_path);
    }
    Ok(get_settings(state).await.unwrap())
}

// change this so that this is returning the html layout to let the client edit the settings.
#[command(async)]
pub async fn edit_settings(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().await;
    let (settings, manager) = join!(app_state.setting.read(), app_state.manager.read());

    let install_path = manager.get_install_path();
    let cache_path = &settings.blend_dir;
    let render_path = &settings.render_dir;

    Ok(html!(
        form tauri-invoke="update_settings" hx-target="this" hx-swap="outerHTML" {
            h3 { "Blender Installation Path:" };
            input name="installPath" class="form-input" readonly="true" tauri-invoke="select_directory" hx-trigger="click" hx-target="this" value=(install_path.to_str().unwrap() );

            h3 { "Blender File Cache Path:" };
            input name="cachePath" class="form-input" readonly="true" tauri-invoke="select_directory" hx-trigger="click" hx-target="this" value=(cache_path.to_str().unwrap());

            h3 { "Render cache directory:" };
            input name="renderPath" class="form-input" readonly="true" tauri-invoke="select_directory" hx-trigger="click" hx-target="this" value=(render_path.to_str().unwrap());
            
            br;
            
            button tauri-invoke="update_settings" { "Save" };
            button tauri-invoke="get_settings" { "Cancel" };
        };
    ).0)
}

#[command(async)]
pub async fn get_settings(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().await;
    let (settings, manager) = join!(app_state.setting.read(), app_state.manager.read());

    let install_path = manager.get_install_path().to_str().unwrap();
    let cache_path = &settings.blend_dir.to_str().unwrap();
    let render_path = &settings.render_dir.to_str().unwrap();

    Ok(html!(
        div tauri-invoke="open_path" hx-target="this" hx-swap="outerHTML" {
            h3 { "Blender Installation Path:" };
            label hx-info=(json!( { "path": install_path } )) { (install_path) };
            
            h3 { "Blender File Cache Path:" };
            label hx-info=(json!( { "path": cache_path } )) { (cache_path) };
            
            h3 { "Render cache directory:" };
            label hx-info=(json!( { "path": render_path } )) { (render_path) };
            br;
            
            button tauri-invoke="edit_settings" { "Edit" };
        }
    )
    .0)
}

#[command]
pub fn setting_page() -> String {
    html! {
        div class="content"  {
            h1 { "Settings" };

            p { r"Here we list out all possible configuration this tool can offer to user.
                    Exposing rich and deep components to customize your workflow" };

            div class="group" id=(SETTING) tauri-invoke="get_settings" hx-trigger="load" hx-target="this" { };
            
            h3 { "Blender Installation" };
            
            button tauri-invoke="add_blender_installation" { "Add from Local Storage" };
            button tauri-invoke="install_from_internet" { "Install version" };
            
            div class="group" {
                table {
                    thead {
                        th { "Version" };
                        th { "Executable Path" };
                    };
                    tbody id="blender-table" tauri-invoke="list_blender_installed" hx-trigger="load blenderUpdate" hx-target="this" { };
                };
            };
        }
    }.0
}
