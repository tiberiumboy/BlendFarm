use super::remote_render::remote_render_page;
use crate::models::{app_state::AppState, job::Job};
use crate::services::tauri_app::{JobAction, UiCommand};
use blender::models::mode::RenderMode;
use futures::channel::mpsc::{self};
use futures::{SinkExt, StreamExt};
use maud::html;
use semver::Version;
use serde_json::json;
use std::{path::PathBuf, str::FromStr};
use tauri::{State, command};
use tokio::sync::Mutex;
use uuid::Uuid;

// input values are always string type. I need to validate input on backend instead of front end.
// return invalidation if the value are not accepted.
#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    start: String,
    end: String,
    version: Version,
    path: PathBuf,
    output: PathBuf,
) -> Result<String, String> {
    let mode = RenderMode::try_new(&start, &end).map_err(|e| e.to_string())?;

    let job = Job {
        mode,
        project_file: path,
        blender_version: version,
        output,
    };

    let add = UiCommand::Job(JobAction::Advertise(job));
    let mut app_state = state.lock().await;
    app_state
        .invoke
        .send(add)
        .await
        .map_err(|e| e.to_string())?;
    Ok(remote_render_page())
}

#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let (sender, mut receiver) = mpsc::channel(0);
    let mut server = state.lock().await;
    let cmd = UiCommand::Job(JobAction::List(sender));
    if let Err(e) = server.invoke.send(cmd).await {
        eprintln!("Fail to send command to server! {e:?}");
    }

    let content = match receiver.select_next_some().await {
        Some(list) => {
            html! {
                @for job in list {
                    div {
                        table {
                            tbody {
                                tr tauri-invoke="get_job" hx-vals=(json!({"jobId":job.id.to_string()})) hx-target="#detail" {
                                    td style="width:100%" {
                                        (job.item.get_file_name())
                                    };
                                };
                            };
                        };
                    };
                };
            }
        }
        None => {
            html! {
                div {
                    // TODO: See about language locales?
                    "No job found!"
                }
            }
        }
    };
    Ok(content.0)
}

fn fetch_img_result(path: &PathBuf) -> Option<Vec<PathBuf>> {
    match path.read_dir() {
        // read the directory content
        Ok(dir) => {
            let mut list = dir
                .filter_map(|res| res.ok()) // collect valid result
                .map(|ent| ent.path()) // collect path from Directory entry result
                .filter(|path| path.extension().map_or(false, |ext| ext == "png"))
                .collect::<Vec<PathBuf>>(); // collect the result into array list
            list.sort(); // the list is not organzied, sort the list after collecting data
            Some(list)
        }
        Err(e) => {
            eprintln!("Unable to find directory! {:?} | {e:?}", &path);
            None
        }
    }
}

fn convert_file_src(path: &PathBuf) -> String {
    #[cfg(any(windows, target_os = "android"))]
    let base = "http://asset.localhost/";
    #[cfg(not(any(windows, target_os = "android")))]
    let base = "asset://localhost/";

    let path = dunce::canonicalize(path).expect("Should be able to canonicalize path!");
    let binding = path.to_string_lossy();
    let encoded = urlencoding::encode(&binding);

    format!("{base}{encoded}")
}

#[command(async)]
pub async fn get_job(state: State<'_, Mutex<AppState>>, job_id: &str) -> Result<String, ()> {
    let (sender, mut receiver) = mpsc::channel(0);
    let job_id = Uuid::from_str(job_id).map_err(|e| {
        eprintln!("Unable to parse uuid? \n{e:?}");
        ()
    })?;

    let mut app_state = state.lock().await;
    let cmd = UiCommand::Job(JobAction::Get(job_id.into(), sender));
    if let Err(e) = app_state.invoke.send(cmd).await {
        eprintln!("{e:?}");
    };

    match receiver.select_next_some().await {
        Some(job) => {
            // TODO: it would be nice to provide ffmpeg gif result of the completed render image.
            // Something to add for immediate preview and feedback from render result
            // this is to fetch the render collection
            let result = fetch_img_result(&job.item.output);

            Ok(html!(
                div {
                        p { "Job Detail" };
                        button tauri-invoke="open_dir" hx-vals=(json!(job.item.project_file.to_str().unwrap())) { ( job.item.project_file.to_str().unwrap() ) };
                        div { ( job.item.output.to_str().unwrap() ) };
                        div { ( job.item.blender_version.to_string() ) };
                        button tauri-invoke="delete_job" hx-vals=(json!({"jobId":job_id})) hx-target="#workplace" { "Delete Job" };
                        p;
                        @if let Some(list) = result {
                            @for img in list {
                                tr {
                                    td {
                                        img width="120px" src=(convert_file_src(&img));
                                    }
                                }
                            }
                        }
                    };
                )
                .0)
        }
        None => Ok(html!(
        div {
                p { "Job do not exist.. How did you get here?" };
            };
        )
        .0),
    }
}

// we'll need to figure out more about this? How exactly are we going to update the job?
#[command(async)]
pub fn update_job() {
    todo!("Figure out the implementation to update the job status for example?");
}

/// just delete the job from database. Notify peers to abandon task matches job_id
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, job_id: &str) -> Result<String, String> {
    // here we're deleting it from the database
    let mut app_state = state.lock().await;
    let id = Uuid::from_str(job_id).map_err(|e| format!("{e:?}"))?;
    let cmd = UiCommand::Job(JobAction::Remove(id));
    if let Err(e) = app_state.invoke.send(cmd).await {
        eprintln!("{e:?}");
    }

    Ok(remote_render_page())
}

#[cfg(test)]
mod test {
    /*
        In this test suite, we are going to simply invoke all of the api function that are exposed to the UI.
        Each API should have at least a minimum 1 passing test and 4 expect failures on certain edge cases
        (malform input entry, wrong json syntax, incomplete form, etc)

        TODO: See about how we can get test coverage that handle all possible cases
    */

    use std::ops::Range;

    use anyhow::Error;
    use super::*;
    use futures::channel::mpsc::Receiver;
    use ntest::timeout;
    use crate::{config_sqlite_db, services::tauri_app::TauriApp};
    use tauri::{test::{mock_builder, MockRuntime}, webview::InvokeRequest};

    async fn scaffold_app() -> Result<(tauri::App<MockRuntime>, Receiver<UiCommand>), Error> {
        let (invoke, receiver) = mpsc::channel(1);
        let conn = config_sqlite_db().await?;
        let app = TauriApp::new(&conn).await;

        let app = app.config_tauri_builder(mock_builder(), invoke).await?;
        Ok((app, receiver))
    }

    #[tokio::test]
    #[timeout(5000)]
    async fn create_job_successfully() {
        // For now I'm going to let this pass, until I figure out how/why mockup tauri app dead-lock on initialization.
        let (app, mut receiver) = scaffold_app().await.unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
        let start = "1".to_owned();
        let end = "2".to_owned();
        let blender_version = Version::new(4, 1, 0);
        let project_file = PathBuf::from("./blender_rs/examples/assets/test.blend".to_owned());
        let output = PathBuf::from("./blender_rs/examples/assets/".to_owned());

        let body = json!({
            "start": start,
            "end": end,
            "version": blender_version,
            "path": project_file,
            "output": output,
        });

        let res = tauri::test::get_ipc_response(&webview, InvokeRequest {
            cmd: "create_job".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        }).map(|b| b.deserialize::<String>().unwrap());

        assert!(res.is_ok());

        let expected_mode = RenderMode::Frame(1);
        let job = Job::new(expected_mode, project_file, blender_version, output);

        let event = receiver.select_next_some().await;
        assert_eq!(event, UiCommand::Job(JobAction::Advertise(job)));
    }

    #[tokio::test]
    #[timeout(5000)]
    async fn create_job_malform_fail() {
        // For now I'm going to let this pass, until I figure out how/why mockup tauri app dead-lock on initialization.
        let (app, _) = scaffold_app().await.unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
        let start = "1".to_owned();
        let end = "2".to_owned();
        let project_file = PathBuf::from("./blender_rs/examples/assets/test.blend".to_owned());
        let output = PathBuf::from("./blender_rs/examples/assets/".to_owned());

        let body = json!({
            "start": start,
            "end": end,
            "version": "1a2b3c",
            "path": project_file,
            "output": output,
        });

        let res = tauri::test::get_ipc_response(&webview, InvokeRequest {
            cmd: "create_job".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        }).map(|b| b.deserialize::<String>().unwrap());

        assert!(res.is_err());
    }

    //#endregion
}
