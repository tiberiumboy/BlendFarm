// this is the settings controller section that will handle input from the setting page.
use crate::models::app_state::AppState;
use blender::blender::Blender;
use maud::html;
use semver::Version;
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

/*
    Because blender installation path is not store in server setting, it is infact store under blender manager,
    we will need to create a new custom response message to provide all of the information needed to display on screen properly
*/

// could this be a utility tool?


/// Add a new blender entry to the system, but validate it first!
#[command(async)]
pub async fn add_blender_installation(
    state: State<'_, Mutex<AppState>>,
    path: PathBuf,
    // TODO: Need to change this to string, string?
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
pub async fn edit_setting_dialog(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().await;
    let _manager = app_state.manager.read().await;
    let _setting = app_state.setting.read().await;
    Ok(html! (
        div id="modal" _="on closeModal add .closing then wait for animationend then remove me" {
            div class="modal-underlay" _="on click trigger closeModal";
            div class="modal-content" { "content" };
        };
    ).into_string())
}

#[command(async)]
pub async fn setting_page(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().await;

    // we can combine these two together.
    let ( server_settings, blender_manager ) = ( app_state.setting.read().await, app_state.manager.read().await);
    let install_path = blender_manager.as_ref().to_owned();
    let cache_path = server_settings.blend_dir.clone();
    let render_path = server_settings.render_dir.clone();
    let mut localblenders = blender_manager.get_blenders().clone();
    localblenders.sort();
    localblenders.reverse();

    // draw and display the setting page here
    Ok(html! {
        div class="content" {
            h1 { "Settings" };
            
            p { r"Here we list out all possible configuration this tool can offer to user.
                    Exposing rich and deep components to customize your workflow" };
            
            // Probably can do a edit form instead?
            div class="group" {
                h3 { "Blender Installation Path:" };
                input id="install_path_id" name="install_path" class="form-input" readonly="true" value=(install_path.to_str().unwrap());
                
                h3 { "Blender File Cache Path:" };
                input id="cache_path_id" name="cache_path" class="form-input" readonly="true" value=(cache_path.to_str().unwrap());
                
                h3 { "Render cache directory:" };
                input id="render_path_id" name="render_path" class="form-input" readonly="true" value=(render_path.to_str().unwrap());

                button tauri-invoke="edit_setting_dialog" hx-target="body" hx-swap="beforeend" { "Edit" };
            };

            h3 { "Blender Installation" };

            button tauri-invoke="installBlenderFromLocal" { "Add from Local Storage" };
            button tauri-invoke="{() => setShowModal(true)}>" { "Install version" };
            div class="group" {
                @for blend in localblenders {
                    div class="item" key=(format!("{}_{}", blend.get_version(), blend.get_executable().to_str().unwrap())) {
                        table {
                            tbody {
                                tr {
                                    td style="width: '100%'" {
                                        (format!("Blender {}", blend.get_version()))
                                    };
                                };
                            };
                        };
                    };
                };
            };
        };
    }.into_string())
}
