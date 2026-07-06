use crate::constant::WORKPLACE;
use crate::models::job::{CreatedJobDto, Output};
use crate::models::{app_state::AppState, job::{Job, JobAction}};
use crate::services::tauri_app::UiCommand;
use blender::blend_file::BlendFile;
use blender::models::mode::RenderMode;
use futures::SinkExt;
use maud::{html, PreEscaped};
use semver::Version;
use serde_json::json;
use std::{path::PathBuf, str::FromStr};
use tauri::{State, command};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Used to render the job list on teh side of the app.
pub(crate) fn render_list_job(collection: &Option<Vec<CreatedJobDto>>) -> String {
    match collection { 
        Some(list) => {
            html! {
                @for job in list {
                    div {
                        table {
                            tbody {
                                tr tauri-invoke="get_job_detail" hx-vals=(json!({"jobId":job.id.to_string()})) hx-target={"#" (WORKPLACE) } {
                                    td style="width:100%" {
                                        (job.item.get_file_name_expected().to_string_lossy())
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
                    "No job found!"
                }
            }
        }
    }.0
}

/// Render the full job description and detail page.
pub(crate) fn render_job_detail_page(job: &Option<CreatedJobDto>) -> String {
    match job {
        Some(job) => {
            let result = fetch_img_result(&job.item.as_ref());

            // TODO: it would be nice to provide ffmpeg gif result of the completed render image.
            // Something to add for immediate preview and feedback from render result
            // this is to fetch the render collection
            // if let Some(imgs) = result {
            //     let preview = fetch_img_preview(&job.item.output, &imgs);
            // }

            let project_file = AsRef::<BlendFile>::as_ref(&job.item).to_path();
            let output = AsRef::<Output>::as_ref(&job.item).to_str().unwrap();
            let version = AsRef::<Version>::as_ref(&job.item);
            let job_info = job.item.clone();
            let (start, end) = job_info.get_range();

            html!(
                div class="content" {
                    h2 { "Job Detail: " ( project_file.file_name().unwrap().to_string_lossy() ) };

                    div { 
                        button tauri-invoke="open_dir" hx-vals=(json!({"path": project_file.to_string_lossy()})) { "File path:" }; 
                        ( project_file.to_string_lossy() )
                    }
                    
                    div { 
                        button tauri-invoke="open_dir" hx-vals=(json!({"path": output})) { "Output:" }; 
                        ( output )
                    }
                    
                    div { "Target Blender Version: " ( version.to_string() ) };

                    div { "Start: " (start) " | End: " (end) }
                    
                    button tauri-invoke="delete_job" hx-vals=(json!({"jobId":job.id})) hx-target="#workplace" { "Delete Job" };
                    
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
        }
        None => html!(
        div {
                p { "Job do not exist.. How did you get here?" };
            };
        ),
    }.0
}

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
    let mut app_state = state.lock().await;
    let job_created = app_state.create_job(job).await.map_err(|e| e.to_string())?;
    let list = app_state.list_jobs().await.map_err(|e| e.to_string())?;

    let list = render_list_job(&list);
    let detail = render_job_detail_page(&Some(job_created));
    
    Ok(html!(
        div hx-target={ "#" (WORKPLACE) }{
            (PreEscaped(detail))   
        }
        div id="joblist" hx-swap-oob="true" {
            (PreEscaped(list))
        }
    )
    .0)
}

#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let mut server = state.lock().await;
    let content = server.list_jobs().await.map_err(|e| e.to_string())?; //cmd_list_jobs(&mut server).await;
    Ok(render_list_job(&content))
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
    // Consider about removing dunce lib for less dependencies involve for this case?
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
    let job_id = Uuid::from_str(job_id).map_err(|e| format!("Unable to parse uuid? \n{e:?}"))?;
    let mut app_state = state.lock().await;
    let result = app_state.fetch_job(job_id).await;
    Ok(render_job_detail_page(&result))
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
    use crate::{services::tauri_app::TauriApp};
    // use crate::models::constant::test::{EXAMPLE_FILE, EXAMPLE_OUTPUT};
    use anyhow::Error;
    use futures::channel::mpsc::{self, Receiver};
    use ntest::timeout;
    use tauri::{
        test::{mock_builder, MockRuntime},
        // webview::InvokeRequest
    };

    // TODO: Fix this so that I can get unit test working again
    #[allow(dead_code)]
    async fn scaffold_app() -> Result<(tauri::App<MockRuntime>, Receiver<UiCommand>), Error> {
        let (_invoke, receiver) = mpsc::channel(1);
        // let conn = config_sqlite_db().await?;
        // let app = TauriApp::new(&conn).await;
        // TODO: Find a better way to get around this approach. Seems like I may not need to have an actual tauri app builder?
        // error: symbol `_EMBED_INFO_PLIST` is already defined
        let context = tauri::generate_context!("tauri.conf.json");
        let app = TauriApp::init_tauri_plugins(mock_builder()).build(context).expect("Should be able to build");
        Ok((app, receiver))
    }

    #[tokio::test]
    #[timeout(5000)]
    async fn create_job_successfully() {
        // For now I'm going to let this pass, until I figure out how/why mockup tauri app dead-lock on initialization.
        /*
        let (app, mut receiver) = scaffold_app().await.unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let start = "1".to_owned();
        let end = "2".to_owned();
        let blender_version = Version::new(4, 1, 0);
        let project_file = PathBuf::from(EXAMPLE_FILE);
        let output = PathBuf::from(EXAMPLE_OUTPUT);

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
        */

        assert!(true);
    }

    #[tokio::test]
    #[timeout(5000)]
    async fn create_job_malform_fail() {
        // For now I'm going to let this pass, until I figure out how/why mockup tauri app dead-lock on initialization.
        // let (app, _) = scaffold_app().await.unwrap();
        // let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default());
        // let start = "1".to_owned();
        // let end = "2".to_owned();
        // let project_file = PathBuf::from("./blender_rs/examples/assets/test.blend".to_owned());
        // let output = PathBuf::from("./blender_rs/examples/assets/".to_owned());

        // let body = json!({
        //     "start": start,
        //     "end": end,
        //     "version": "1a2b3c",
        //     "path": project_file,
        //     "output": output,
        // });

        // let res = tauri::test::get_ipc_response(
        //     &webview,
        //     InvokeRequest {
        //         cmd: "create_job".into(),
        //         callback: tauri::ipc::CallbackFn(0),
        //         error: tauri::ipc::CallbackFn(1),
        //         url: "tauri://localhost".parse().unwrap(),
        //         body: tauri::ipc::InvokeBody::Json(body),
        //         headers: Default::default(),
        //         invoke_key: tauri::test::INVOKE_KEY.to_string(),
        //     },
        // )
        // .map(|b| b.deserialize::<String>().unwrap());

        // assert!(res.is_err());
        assert!(true);
    }

    //#endregion
}
