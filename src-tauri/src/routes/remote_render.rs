/* Dev blog:
- I really need to draw things out and make sense of the workflow for using this application.
I wonder why initially I thought of importing the files over and then selecting the files again to begin the render job?

For now - Let's go ahead and save the changes we have so far.
Next update - Remove Project list, and instead just allow user to create a new job.
when you create a new job, it immediately sends a new job to the server farm

for future features impl:
Get a preview window that show the user current job progress - this includes last frame render, node status, (and time duration?)
*/
use crate::AppState;
use blender::blender::Blender;
use build_html::{Html, HtmlElement, HtmlTag};
use maud::html;
use semver::Version;
use std::path::PathBuf;
use tauri::{command, AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::sync::Mutex;

// todo break commands apart, find a way to get the list of versions
async fn list_versions(app_state: &AppState) -> Vec<Version> {
    let manager = app_state.manager.read().await;
    let mut versions = Vec::new();

    let _ = manager.home.as_ref().iter().for_each(|b| {
        let version = match b.fetch_latest() {
            Ok(download_link) => download_link.get_version().clone(),
            Err(_) => Version::new(b.major, b.minor, 0),
        };
        versions.push(version);
    });

    // let manager = server.manager.read().await;
    let _ = manager
        .get_blenders()
        .iter()
        .for_each(|b| versions.push(b.get_version().clone()));

    versions
}

/// List all of the available blender version.
#[command(async)]
pub async fn available_versions(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let mut root = HtmlElement::new(HtmlTag::Div);
    let server = state.lock().await;
    let versions = list_versions(&server).await;

    versions.iter().for_each(|b| {
        root.add_child(
            HtmlElement::new(HtmlTag::ListElement)
                .with_child(b.to_string().into())
                .into(),
        );
    });

    Ok(root.to_html_string())
}

#[command(async)]
pub async fn create_new_job(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<String, String> {
    // tell tauri to open file dialog
    // with that file path we will run import_blend function.
    // else return nothing.
    let result = match app
        .dialog()
        .file()
        .add_filter("Blender", &["blend"])
        .blocking_pick_file()
    {
        Some(file_path) => match file_path {
            FilePath::Path(path) => import_blend(state, path).await.unwrap(),
            FilePath::Url(uri) => import_blend(state, uri.as_str().into()).await.unwrap(),
        },
        None => "".to_owned(),
    };

    Ok(result)
}

// change this to return HTML content of the info back.
#[command(async)]
pub async fn import_blend(
    state: State<'_, Mutex<AppState>>,
    path: PathBuf,
) -> Result<String, String> {
    let server = state.lock().await;
    let versions = list_versions(&server).await;

    if path.file_name() == None {
        return Err("Should be a valid file!".to_owned());
    }

    let data = match Blender::peek(&path).await {
        Ok(data) => data,
        Err(e) => return Err(e.to_string()),
    };

    // _ = "on closeModal add .closing then wait for animationend then remove me" // Hyperscript
    let content = html! {
        div id="modal" _="on closeModal add .closing then wait for animationend then remove me" {
            div class="modal-underlay" _="on click trigger closeModal";
            div class="modal-content" {
                form method="dialog" tauri-invoke="create_job" {
                    h1 { "Create new Render Job" };
                    label { "Project File Path:" };
                    input type="text" class="form-input" name="path" value=(path.to_str().unwrap()) placeholder="Project path" readonly={true};
                    // add a button here to let the user search by directory path. Let them edit the form.
                    br;
                    label "Blender Version:";
                    select name="version" value=(data.last_version) {
                        @for i in versions {
                            option value=(i) { (i) }
                        }
                    };

                    div name="mode" {
                        label id="frameStartLabel" htmlFor="start" { "Start" };
                        input class="form-input" name="start" type="number" value=(data.frame_start);
                        label id="frameEndLabel" htmlFor="end" { "End" };
                        input class="form-input" name="end" type="number" value=(data.frame_end);
                    };

                    label { "Output destination:" };
                    input type="text" class="form-input" placeholder="Output Path" name="output" value=(data.output.to_str().unwrap()) readonly="true";
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
pub async fn remote_render_page(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let jobs = server.job_db.read().await;
    let job_list = jobs.list_all().await.unwrap();

    let content = html! {
        div class="content" {
            h1 { "Remote Jobs" };

            button tauri-invoke="create_new_job" hx-target="body" hx-swap="beforeend" {
                "Import"
            };

            div class="group" {
                @for job in job_list {
                    div {
                        table {
                            tbody {
                                tr tauri-invoke="job_detail" hx-target="#detail" {
                                    td style="width:100%" {
                                        (job.get_file_name())
                                    };
                                };
                            };
                        };
                    };
                };
            };

            div id="detail";
        };
    };

    Ok(content.into_string())
}
