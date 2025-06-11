/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use super::util::select_directory;
use crate::models::app_state::AppState;
use blender::blender::Blender;
use maud::html;
use semver::Version;
use std::path::PathBuf;
use tauri::{command, AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::sync::Mutex;

// todo break commands apart, find a way to get the list of versions without using appstate?
async fn list_versions(app_state: &AppState) -> Vec<Version> {
    // TODO: see if there's a better way to get around this problematic function
    /*
       Issues: I'm noticing a significant delay of behaviour event happening here when connected online.
       When connected online, BlenderManager seems to hold up to approximately 2-3 seconds before the remaining content fills in.
       Offline loads instant, which is exactly the kind of behaviour I wanted to use for this application.
    */
    let manager = app_state.manager.write().await;
    let mut versions = Vec::new();

    // fetch local installation first.
    let mut local = manager
        .get_blenders()
        .iter()
        .map(|b| b.get_version().clone())
        .collect::<Vec<Version>>();

    if !local.is_empty() {
        versions.append(&mut local);
    }

    // then display the rest of the download list
    if let Some(downloads) = manager.fetch_download_list() {
        let mut item = downloads
            .iter()
            .map(|d| d.get_version().clone())
            .collect::<Vec<Version>>();
        versions.append(&mut item);
    };

    versions
}

/// List all of the available blender version.
#[command(async)]
pub async fn available_versions(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let versions = list_versions(&server).await;

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
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<String, String> {
    let path = match app
        .dialog()
        .file()
        .add_filter("Blender", &["blend"])
        .blocking_pick_file()
    {
        Some(file_path) => match file_path {
            FilePath::Path(path) => path,
            FilePath::Url(uri) => uri.as_str().into(),
        },
        None => return Err("No file selected".into()),
    };
    import_blend(state, path).await
}

#[command(async)]
pub async fn update_output_field(app: AppHandle) -> Result<String, ()> {
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
    let server = state.lock().await;
    // for some reason this function takes longer online than it does offline?
    let versions = list_versions(&server).await;

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

#[command(async)]
pub async fn remote_render_page() -> Result<String, String> {
    let content = html! {
        div class="content" {
            h1 { "Remote Jobs" };

            button tauri-invoke="create_new_job" hx-target="body" hx-indicator="#spinner" hx-swap="beforeend" {
                "Import"
            };

            img id="spinner" class="htmx-indicator" src="/assets/svg-loaders/tail-spin.svg";

            div class="group" id="joblist" tauri-invoke="list_jobs" hx-trigger="load" hx-target="this" {
            };

            div id="detail";
        };
    };

    Ok(content.0)
}
