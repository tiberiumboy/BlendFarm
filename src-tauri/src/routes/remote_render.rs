/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use super::util::select_directory;
use crate::{
    models::app_state::AppState,
    services::tauri_app::{BlenderAction, UiCommand},
};
use blender::blender::Blender;
use futures::{SinkExt, StreamExt, channel::mpsc};
use maud::html;
use semver::Version;
use std::path::PathBuf;
use tauri::{AppHandle, State, command};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::sync::Mutex;

// todo break commands apart, find a way to get the list of versions without using appstate?
// we're using appstate to access invoker commands. the invoker needs to send us info
async fn list_versions(app_state: &mut AppState) -> Vec<Version> {
    // TODO: see if there's a better way to get around this problematic function
    /*
       Issues: I'm noticing a significant delay of behaviour event happening here when connected online.
       When connected online, BlenderManager seems to hold up to approximately 2-3 seconds before the remaining content fills in.
       Offline loads instant, which is exactly the kind of behaviour I expect to see from this application.
    */
    let (sender, mut receiver) = mpsc::channel(1);
    let event = UiCommand::Blender(BlenderAction::List(sender));
    if let Err(e) = app_state.invoke.send(event).await {
        eprintln!("Fail to send event! {e:?}");
        return Vec::new();
    }

    let res = receiver.select_next_some().await;
    match res {
        // Clone operation used here. might be expensive? See if there's another way to get aorund this.
        Some(list) => list
            .iter()
            .map(|f| f.get_version().clone())
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

/// Ask Tauri to display ui blocking dialog and return file path to blender.
/// This function will read the file and display another dialog prompt for additional detail before continue to display the result from import_blend()
#[command(async)]
pub async fn create_new_job(
    handle: State<'_, Mutex<AppHandle>>,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let app = handle.lock().await;
    let given_path = app
        .dialog()
        .file()
        .add_filter("Blender", &["blend"])
        .blocking_pick_file()
        .and_then(|f| match f {
            FilePath::Path(f) => Some(f),
            FilePath::Url(u) => Some(u.as_str().into()),
        });

    if let Some(path) = given_path {
        return import_blend(state, path).await;
    }
    Err("No file selected!".to_owned())
}

#[command]
pub async fn update_output_field(app: State<'_, Mutex<AppHandle>>) -> Result<String, ()> {
    match select_directory(app).await {
        Ok(path) => Ok(html!(
            input type="text" class="form-input" placeholder="Output Path" name="output" value=(path) readonly={true};
        ).0),
        Err(_) => Err(()),
    }
}

// change this to return HTML content of the info back.
#[command(async)]
pub async fn import_blend(
    state: State<'_, Mutex<AppState>>,
    path: PathBuf,
) -> Result<String, String> {
    // for some reason this function takes longer online than it does offline?
    // TODO: set unit test to make sure this function doesn't repetitively call blender.org everytime it's called.
    let mut app_state = state.lock().await;
    let versions = list_versions(&mut app_state).await;

    if path.file_name() == None {
        return Err("Should be a valid file!".to_owned());
    }

    let data = match Blender::peek(&path).await {
        Ok(data) => data,
        Err(e) => return Err(e.to_string()),
    };

    let content = html! {
        div id="modal" _="on closeModal add .closing then wait for animationend then remove me" {
            div class="modal-underlay" _="on click trigger closeModal" {};
            div class="modal-content" {
                form method="dialog" tauri-invoke="create_job" hx-target="#workplace" _="on submit trigger closeModal" {
                    h1 { "Create new Render Job" };
                    label { "Project File Path:" };
                    input type="text" class="form-input" name="path" value=(path.to_str().unwrap()) placeholder="Project path" readonly={true};
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

#[command]
pub fn remote_render_page() -> String {
    html! {
        div class="content" {
            h1 { "Remote Jobs" };

            button tauri-invoke="create_new_job" hx-target="body" hx-indicator="#spinner" hx-swap="beforeend" {
                "Import"
            };

            img id="spinner" class="htmx-indicator" src="/assets/svg-loaders/tail-spin.svg";

            // Is there a way to select the first item on the list by default?
            div class="group" id="joblist" tauri-invoke="list_jobs" hx-trigger="load" hx-target="this" {
            };

            div id="detail";
        };
    }.0
}
