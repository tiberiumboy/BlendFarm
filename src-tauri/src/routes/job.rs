use crate::models::{app_state::AppState, job::Job};
use crate::services::tauri_app::{JobAction, UiCommand, WORKPLACE};
use blender::models::mode::RenderMode;
use futures::channel::mpsc::{self};
use futures::{SinkExt, StreamExt};
use maud::{html, PreEscaped};
use semver::Version;
use serde_json::json;
// use std::process::Command;
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
    let job = Job::from(mode, path, version, output).map_err(|e| e.to_string())?; 
    let (sender, mut receiver) = mpsc::channel(1);
    let add = UiCommand::Job(JobAction::Create(job, sender));
    let mut app_state = state.lock().await;
    app_state
        .invoke
        .send(add)
        .await
        .map_err(|e| e.to_string())?;

    // TODO: Finish implementing handling job receiver here.
    let result = receiver.select_next_some().await;
    // TODO: Find a way to handle this error or not?
    let _ = dbg!(result);
    // TODO: Utilize hx-swap-oob to update the list, then we'll update the portal to display selected job.

    Ok(html!(
        div {
            "TODO: Figure out what needs to get added here"
        }
    )
    .0)
}

#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let (sender, mut receiver) = mpsc::channel(0);
    let mut server = state.lock().await;
    let cmd = UiCommand::Job(JobAction::All(sender));
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
                                tr tauri-invoke="get_job_detail" hx-vals=(json!({"jobId":job.id.to_string()})) hx-target={"#" (WORKPLACE) } {
                                    td style="width:100%" {
                                        (job.item.get_file_name_expected())
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
            eprintln!("Unable to find any image stored in the directory:\nPath:{path:?}\nError:{e:?}");
            None
        }
    }
}

/*
fn fetch_img_preview(path: &PathBuf, imgs: &Vec<PathBuf>) -> PathBuf {
    // ffmpeg command usage
    // ffmpeg -y -framerate 10 -i <image>%02d.png -s 426x240 preview.gif

    let output = Command::new("ffmpeg").arg("-y -framerate 10 -i 02d.png -s 426x240 preview.gif").output();


    PathBuf::new()
}
*/

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
pub async fn get_job_detail(
    state: State<'_, Mutex<AppState>>,
    job_id: &str,
) -> Result<String, String> {
    let (sender, mut receiver) = mpsc::channel(0);
    let job_id = Uuid::from_str(job_id).map_err(|e| format!("Unable to parse uuid? \n{e:?}"))?;

    let mut app_state = state.lock().await;
    let cmd = UiCommand::Job(JobAction::Find(job_id.into(), sender));
    if let Err(e) = app_state.invoke.send(cmd).await {
        eprintln!("Fail to send job action: {e:?}");
    };

    match receiver.select_next_some().await {
        Some(job) => {
            let result = fetch_img_result(&job.item.get_output());

            // TODO: it would be nice to provide ffmpeg gif result of the completed render image.
            // Something to add for immediate preview and feedback from render result
            // this is to fetch the render collection
            // if let Some(imgs) = result {
            //     let preview = fetch_img_preview(&job.item.output, &imgs);
            // }

            Ok(html!(
                div class="content" {
                        h2 { "Job Detail" };

                        button tauri-invoke="open_dir" hx-vals=(json!(job.item.get_project_path().to_str().unwrap())) { ( job.item.get_project_path().to_str().unwrap() ) };
                        
                        div { ( job.item.get_output().to_str().unwrap() ) };
                        
                        div { ( job.item.get_version().to_string() ) };
                        
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
                        @else {
                            div {
                                "No image found in output directory..."
                            }
                        }
                    };
                )
                .0)
        }
        None => Err(html!(
        div {
                p { "Job do not exist.. How did you get here?" };
            };
        )
        .0),
    }
}

// we'll need to figure out more about this? How exactly are we going to update the job?
#[command(async)]
pub async fn update_job(state: State<'_, Mutex<AppState>>, job_id: Uuid) -> Result<(), String> {
    let mut app_state = state.lock().await;
    if let Err(e) = app_state.invoke.send(UiCommand::Job(JobAction::Kill(job_id))).await {
        return Err(format!("Fail to send command to host! Are you sure this app is responsive? {e:?}").into());
    }

    // TODO: call list_jobs and perform hx-swap-oob here to trigger job list refresh.
    Ok(())
}

/// just delete the job from database. Notify peers to abandon task matches job_id
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, job_id: &str) -> Result<String, String> {
    // here we're deleting it from the database
    {
        let mut app_state = state.lock().await;
        let id = Uuid::from_str(job_id).map_err(|e| format!("{e:?}"))?;
        let cmd = UiCommand::Job(JobAction::Kill(id));
        if let Err(e) = app_state.invoke.send(cmd).await {
            eprintln!("{e:?}");
        }
    }

    // now here we need to refresh the list
    let list = list_jobs(state).await?;
    
    // TODO: do not send back Ok() response if there's an error, consider handling this separately.
    // use a match condition to avoid sending error to the list
    Ok(html!(
        div class="group" id="joblist" hx-swap-oob="true" {
            (PreEscaped(list));
        }
    )
    .0)
}

#[cfg(test)]
mod test {
    /*
        In this test suite, we are going to simply invoke all of the api function that are exposed to the UI.
        Each API should have at least a minimum 1 passing test and 4 expect failures on certain edge cases
        (malform input entry, wrong json syntax, incomplete form, etc)

        TODO: See about how we can get test coverage that handle all possible cases
    */

    use super::*;
    use crate::{config_sqlite_db, services::tauri_app::TauriApp};
    use anyhow::Error;
    use futures::channel::mpsc::Receiver;
    use ntest::timeout;
    use tauri::{
        test::{MockRuntime, mock_builder},
        webview::InvokeRequest,
    };

    async fn scaffold_app() -> Result<(tauri::App<MockRuntime>, Receiver<UiCommand>), Error> {
        let (invoke, receiver) = mpsc::channel(1);
        // let conn = config_sqlite_db().await?;
        // let app = TauriApp::new(&conn).await;
        // TODO: Find a better way to get around this approach. Seems like I may not need to have an actual tauri app builder?
        let app = TauriApp::init_tauri_plugins(mock_builder())?;
        Ok((app, receiver))
    }

    #[tokio::test]
    #[timeout(5000)]
    async fn create_job_successfully() {
        // For now I'm going to let this pass, until I figure out how/why mockup tauri app dead-lock on initialization.
        let (app, mut receiver) = scaffold_app().await.unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
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

        let res = tauri::test::get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: "create_job".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<String>().unwrap());

        assert!(res.is_ok());

        let expected_mode = RenderMode::Frame(1);
        let job = Job::from(expected_mode, project_file, blender_version, output).expect("Should not fail");

        let event = receiver.select_next_some().await;
        let (mock_sender, _) = mpsc::channel(0);
        assert_eq!(event, UiCommand::Job(JobAction::Create(job, mock_sender)));
    }

    #[tokio::test]
    #[timeout(5000)]
    async fn create_job_malform_fail() {
        // For now I'm going to let this pass, until I figure out how/why mockup tauri app dead-lock on initialization.
        let (app, _) = scaffold_app().await.unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
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

        let res = tauri::test::get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: "create_job".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<String>().unwrap());

        assert!(res.is_err());
    }

    //#endregion
}
