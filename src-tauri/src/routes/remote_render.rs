/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use super::util::select_directory;
use crate::models::blender_action::BlenderAction;
use crate::{
    models::app_state::AppState,
    services::tauri_app::{QueryMode, UiCommand},
};
use blender::blend_file::BlendFile;
use blender::blender::Blender;
use futures::{SinkExt, StreamExt, channel::mpsc};
use maud::html;
use semver::Version;
use std::path::PathBuf;
use tauri::{AppHandle, State, command};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::sync::Mutex;

// TODO: where is this function called?
async fn list_versions(app_state: &mut AppState) -> Vec<Version> {
    // TODO: see if there's a better way to get around this problematic function
    /*
       Issues: I'm noticing a significant delay of behaviour event happening here when connected online.
       When connected online, BlenderManager seems to hold up to approximately 2-3 seconds before the remaining content fills in.
       Offline loads instant, which is exactly the kind of behaviour I expect to see from this application.
    */
    let (sender, mut receiver) = mpsc::channel(1);
    let event = UiCommand::Blender(BlenderAction::List(
        sender,
        QueryMode::ONLINE | QueryMode::LOCAL,
    ));
    if let Err(e) = app_state.invoke.send(event).await {
        eprintln!("Fail to send event! {e:?}");
        return Vec::new();
    }

    let res = receiver.select_next_some().await;
    match res {
        // Clone operation used here. might be expensive? See if there's another way to get aorund this.
        Some(list) => list
            .iter()
            .map(|f| f.version.clone())
            .collect::<Vec<Version>>(),
        None => Vec::new(),
    }

    // let mut versions = Vec::new();

    // // fetch local installation first.
    // let mut local = manager
    //     .get_blenders()
    //     .iter()
    //     .map(|b| b.get_version().clone())
    //     .collect::<Vec<Version>>();

    // if !local.is_empty() {
    //     versions.append(&mut local);
    // }

    // // then display the rest of the download list
    // if let Some(downloads) = manager.fetch_download_list() {
    //     let mut item = downloads
    //         .iter()
    //         .map(|d| d.get_version().clone())
    //         .collect::<Vec<Version>>();
    //     versions.append(&mut item);
    // };

    // versions
}

/// List all of the available blender version.
// TODO: not used in the function yet?
#[command(async)]
pub async fn available_versions(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let mut server = state.lock().await;
    let versions = list_versions(&mut server).await;

    Ok(html!(
        div {
            @for version in versions {
                li {
                    (version)
                }
            }
        }
    )
    .0)
}

// This function must be async to avoid ui thread lock. Without async, no dialog will appear and app will freeze
/// Display dialog and return file path to blender.
/// This function will read the file and display another dialog prompt for additional detail before continue to display the result from import_blend()
#[command]
pub async fn open_dialog_for_blend_file(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let given_path = app
        .dialog()
        .file()
        .add_filter("Blender", &["blend"])
        .blocking_pick_file()
        .and_then(|f| match f {
            // TODO - see about converting PathBuf into &str, to reduce .into() for Url
            FilePath::Path(f) => Some(f),
            FilePath::Url(u) => Some(u.as_str().into()),
        });

    if let Some(path) = given_path {
        return import_blend(&state, path).await;
    }
    Err("No file selected!".into())
}

#[command]
pub async fn update_output_field(app: AppHandle) -> Result<String, ()> {
    match select_directory(app).await {
        Ok(path) => Ok(html!(
            input type="text" class="form-input" placeholder="Output Path" name="output" value=(path) readonly={true};
        ).0),
        Err(_) => Err(()),
    }
}

// TODO: Rename this function to "read_blend_file_content" - return info about this file.
// we can multi-purpose this for drag and drop feature
pub async fn import_blend(state: &Mutex<AppState>, path: PathBuf) -> Result<String, String> {
    // for some reason this function takes longer online than it does offline?
    // TODO: set unit test to make sure this function doesn't repetitively call blender.org everytime it's called.
    let mut app_state = state.lock().await;
    let versions = list_versions(&mut app_state).await;

    // validate file path.
    let blend_file = BlendFile::new(&path).map_err(|e| e.to_string())?;
    let data = blend_file.peek_response(None);

    let content = html! {
        div id="modal" _="on closeModal add .closing then wait for animationend then remove me" {
            div class="modal-underlay" _="on click trigger closeModal" {};
            div class="modal-content" {
                form method="dialog" tauri-invoke="create_job" hx-target="#workplace" _="on submit trigger closeModal" {
                    h1 { "Create new Render Job" };
                    label { "Project File Path:" };
                    // TODO: Figure out what this value was suppose to be? What method did this invoke to?
                    // input type="text" class="form-input" name="path" value=(blend_file.to_str().unwrap()) placeholder="Project path" readonly={true};
                   p { "Need to update this method. Please see the source code" }
                   br;

                    label { "Output destination:" };
                    div tauri-invoke="update_output_field" hx-target="this" {
                        input type="text" class="form-input" placeholder="Output Path" name="output" value=(data.current.render_setting.get_output().to_str().unwrap()) readonly={true};
                    }
                    br;

                    div name="mode" {
                        table {
                            tr {
                                th {
                                    label id="versionLabel" htmlfor="version" { "Version" };
                                }
                                th {
                                    label id="frameStartLabel" htmlfor="start" { "Start" };
                                };
                                th {
                                    label id="frameEndLabel" htmlfor="end" { "End" };
                                };
                            };
                            tr {
                                td {
                                    select name="version" value=(data.last_version) style={"width:100%; height:100%;"} {
                                        @for i in versions {
                                            option value=(i) { (i) }
                                        }
                                    };
                                }
                                td style="width:33%" {
                                    input class="form-input" name="start" type="number" value=(data.frame_start);
                                };
                                td style="width:33%" {
                                    input class="form-input" name="end" type="number" value=(data.frame_end);
                                };
                            };
                        };
                    };

                    menu {
                        button type="button" value="cancel" _="on click trigger closeModal" { "Cancel" };
                        button type="submit" { "Ok" };
                    };
                }
            }
        }
    };

    Ok(content.into_string())
}

#[cfg(test)]
mod test {
    // TODO: fill testing suite for this route
}
