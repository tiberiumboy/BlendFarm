use crate::{models::{app_state::AppState, server_setting::ServerSetting}, services::tauri_app::{BlenderAction, QueryMode, SettingsAction, UiCommand}};
use std::{env, path::PathBuf, str::FromStr, process::Command};
use blender::blender::Blender;
use futures::{channel::mpsc, SinkExt, StreamExt};
use maud::html;
use semver::Version;
use serde_json::json;
use tauri::{command, AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::sync::Mutex;

const SETTING: &str= "settings";

#[command]
pub fn open_dir(path: &str) -> Result<(),()> {
    // macos is special, the path link inside app bundle, but cannot access via file explore/finder
    let path = PathBuf::from_str(path).unwrap();
    let result = match env::consts::OS {
        "windows" => Ok("explorer"),
        "macos" => Ok("open"),  
        "linux" => Ok("xdg-open"),
        _ => Err(())
    };
    if let Ok(program) = result {
        Command::new(program)
        .arg(path)
        .spawn()
        .unwrap();
    }
    Ok(())
}

#[command(async)]
pub async fn list_blender_installed(state: State<'_, Mutex<AppState>>) -> Result<String, ()> {
    let (sender, mut receiver) = mpsc::channel(0);
    let mut app_state = state.lock().await;
    
    let event = UiCommand::Blender(BlenderAction::List(sender, QueryMode::LOCAL));
    if let Err(e) = app_state.invoke.send(event).await {
        eprintln!("fail to send mpsc to event! {e:?}");
        return Err(())
    }

    let list = receiver.select_next_some().await.expect("Should expect data back!");

    Ok(html! {
        @for blend in list {
            tr {
                td {
                    label title=(blend.link()) {
                        (blend.version.to_string())
                    }
                };
                td {
                    button tauri-invoke="open_dir" hx-vals=(json!({"path":blend.link()})) {
                        r"📁"
                    }
                    button tauri-invoke="delete_blender" hx-vals=(json!({"path":blend.link() })) 
                    {
                        r"🗑︎"
                    }
                }
            };
        };
    }
    .0)
}

/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub async fn add_blender_installation(
    handle: State<'_, Mutex<AppHandle>>,
    state: State<'_, Mutex<AppState>>, 
) -> Result<(), ()> { // TODO: Need to change this to string, string?
    let app = handle.lock().await;
    let path = match app.dialog().file().blocking_pick_file() {
        Some(file_path) => match file_path {
            FilePath::Path(path) => path,
            FilePath::Url(url) => url.to_file_path().unwrap(),
        },
        None => return Err(()),
    };

    let mut app_state = state.lock().await;
    if let Err(e) = app_state.invoke.send(UiCommand::Blender(BlenderAction::Add(path))).await {
        eprintln!("Fail to send data back! {e:?}");
    }
    Ok(())
}

// So this can no longer be a valid api call?
// TODO: Reconsider refactoring this so that it's not a public api call. Deprecate/remove asap
#[command(async)]
pub async fn fetch_blender_installation(
    state: State<'_, Mutex<AppState>>,
    version: &str,
) -> Result<Blender, ()> {
    let version = Version::parse(version).map_err(|_| ())?;
    let (sender, mut receiver) = mpsc::channel(1);
    let event = UiCommand::Blender(BlenderAction::Get(version, sender));
    let mut app_state = state.lock().await;
    app_state.invoke.send(event).await.unwrap();
    let result = receiver.select_next_some().await;
    
    // let blender = manager.fetch_blender(&version).map_err(|e| match e {
    //     blender::manager::ManagerError::DownloadNotFound { arch, os, url } => {
    //         format!("Download link not found! {arch} {os} {url}")
    //     }
    //     blender::manager::ManagerError::RequestError(request) => {
    //         format!("Request error: {request}")
    //     }
    //     blender::manager::ManagerError::FetchError(fetch) => format!("Fetch error: {fetch}"),
    //     blender::manager::ManagerError::IoError(io) => format!("IoError: {io}"),
    //     blender::manager::ManagerError::UnsupportedOS(os) => format!("Unsupported OS {os}"),
    //     blender::manager::ManagerError::UnsupportedArch(arch) => {
    //         format!("Unsupported architecture! {arch}")
    //     }
    //     blender::manager::ManagerError::UnableToExtract(ctx) => {
    //         format!("Unable to extract content! {ctx}")
    //     }
    //     blender::manager::ManagerError::UrlParseError(url) => format!("Url parse error: {url}"),
    //     blender::manager::ManagerError::PageCacheError(cache) => {
    //         format!("Page cache error! {cache}")
    //     }
    //     blender::manager::ManagerError::BlenderError { source } => {
    //         format!("Blender error: {source}")
    //     }
    // })?;
    
    match result {
        Some(blend) => Ok(blend),
        None => Err(())
    }
}

#[command]
pub fn delete_blender(_path: &str) -> Result<(), ()> {
    todo!("Impl function to delete blender and its local contents");
}

/// - Severe local path to blender from registry (Orphan on disk/not touched)
#[command(async)]
pub async fn disconnect_blender_installation(
    state: State<'_, Mutex<AppState>>,
    blender: Blender,
) -> Result<(), String> {
    let mut app_state = state.lock().await;
    
    let event = UiCommand::Blender(BlenderAction::Disconnect(blender));
    if let Err(e) = app_state.invoke.send(event).await {
        eprintln!("Fail to send blender action event! {e:?}");
        return Err(e.to_string())
    }
    
    Ok(())
}

/// - Delete blender content completely (erasing from disk)
#[command(async)]
pub async fn uninstall_blender(
    state: State<'_, Mutex<AppState>>,
    blender: Blender
) -> Result<(), String>{ 
    // this is where we enter the danger territory of deleting local installation of blender and the file associated with.
    let mut app_state = state.lock().await;

    let event = UiCommand::Blender(BlenderAction::Remove(blender));
    if let Err(e) = app_state.invoke.send(event).await {
        eprintln!("Fail to send blender action event! {e:?}");
        return Err(e.to_string())
    }
    
    Ok(())
}

// I am a little confused about this function.
#[command(async)]
pub async fn update_settings(
    state: State<'_, Mutex<AppState>>,
    install_path: String,
    cache_path: String,
    render_path: String,
) -> Result<(), ()> {
    let _install_path = PathBuf::from(install_path);
    let blend_dir = PathBuf::from(cache_path);
    let render_dir = PathBuf::from(render_path);

    let mut state = state.lock().await;
    let new_setting = ServerSetting {
        blend_dir,
        render_dir,
    };

    let command = UiCommand::Settings(SettingsAction::Update(new_setting));
    if let Err(e) = state.invoke.send(command).await {
        eprintln!("{e:?}");
    }
    Ok(())
}

// change this so that this is returning the html layout to let the client edit the settings.
#[command(async)]
pub async fn edit_settings(state: State<'_, Mutex<AppState>>) -> Result<String, String> {

    let mut app_state = state.lock().await;
    let settings = app_state.get_settings().await.map_err(|e| e.to_string())?;
    
    // let install_path = manager.get_install_path();
    let cache_path = &settings.blend_dir;
    let render_path = &settings.render_dir;

    Ok(html!(
        form tauri-invoke="update_settings" hx-target="this" hx-swap="outerHTML" {
            // h3 { "Blender Installation Path:" };
            // input name="installPath" class="form-input" readonly="true" tauri-invoke="select_directory" hx-trigger="click" hx-target="this" value=(install_path.to_str().unwrap() );

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
    let mut app_state = state.lock().await;
    let settings = app_state.get_settings().await.map_err(|e| e.to_string())?;

    let cache_path = &settings.blend_dir.to_str().unwrap();
    let render_path = &settings.render_dir.to_str().unwrap();

    Ok(html!(
        div tauri-invoke="open_path" hx-target="this" hx-swap="outerHTML" {
            // TODO: Could we make a factory to build buttons for this?
            h3 { "Blender File Cache Path:" };
            button tauri-invoke="open_dir" hx-vals=(json!({"path":cache_path})) {
                r"📁"
            }
            label word-wrap="break-word" hx-info=(json!( { "path": cache_path } )) { (cache_path) };
            
            h3 { "Render cache directory:" };
            button tauri-invoke="open_dir" hx-vals=(json!({"path":render_path})) {
                r"📁"
            }
            label word-wrap="break-word" hx-info=(json!( { "path": render_path } )) { (render_path) };
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
                    tbody id="blender-table" tauri-invoke="list_blender_installed" hx-trigger="load" hx-target="this" { };
                };
            };
        }
    }.0
}
