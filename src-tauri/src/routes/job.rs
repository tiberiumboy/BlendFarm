use super::remote_render::remote_render_page;
use crate::models::{app_state::AppState, job::Job};
use crate::services::tauri_app::UiCommand;
use blender::models::mode::RenderMode;
use futures::channel::mpsc::{self, Sender};
use futures::{SinkExt, StreamExt};
use maud::html;
use semver::Version;
use serde_json::json;
use std::{ops::Range, path::PathBuf, str::FromStr};
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
    let mut app_state = state.lock().await;
    _create_job(&start, &end, version, path, output, &mut app_state.invoke).await
}

// Internal use of the function - useful to perform unit test. outside of public api
// I would like to find a way to use validation for Range somehow?
async fn _create_job(
    start: &str,
    end: &str,
    blender_version: Version,
    project_file: PathBuf,
    output: PathBuf,
    sender: &mut Sender<UiCommand>,
) -> Result<String, String> {
    let mut start = start.parse::<i32>().map_err(|e| e.to_string())?;
    let mut end = end.parse::<i32>().map_err(|e| e.to_string())?;
    // stop if the parser fail to parse.

    // start needs to be the lowest number of all. If it's backward, flip it around.
    if start > end {
        (start, end) = (end, start);
    }

    let mode = if start + 1 == end {
        RenderMode::Frame(start)
    } else {
        RenderMode::Animation(Range { start, end })
    };

    // create a container to hold job info
    let job = Job {
        mode,
        project_file,
        blender_version,
        output,
    };

    let add = UiCommand::AddJobToNetwork(job);
    sender.send(add).await.map_err(|e| e.to_string())?;
    remote_render_page().await
}

#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let (sender, mut receiver) = mpsc::channel(0);
    // using scope to drop mutex sharable state. It must have been waiting for this to go out of scope.
    {
        let mut server = state.lock().await;
        let cmd = UiCommand::ListJobs(sender);
        if let Err(e) = server.invoke.send(cmd).await {
            eprintln!("Fail to send command to server! {e:?}");
        }
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
    let cmd = UiCommand::GetJob(job_id.into(), sender);
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
    {
        // here we're deleting it from the database
        let mut app_state = state.lock().await;
        let id = Uuid::from_str(job_id).map_err(|e| format!("{e:?}"))?;
        let cmd = UiCommand::RemoveJob(id);
        if let Err(e) = app_state.invoke.send(cmd).await {
            eprintln!("{e:?}");
        }
    }

    remote_render_page().await
}

#[cfg(test)]
mod test {
    /*
        In this test suite, we are going to simply invoke all of the api function that are exposed to the UI.
        Each API should have at least a minimum 1 passing test and 4 expect failures on certain edge cases
        (malform input entry, wrong json syntax, incomplete form, etc)

        TODO: See about how we can get test coverage that handle all possible cases
    */

    //#region create_jobs

    use blender::manager::Manager;
    use futures::channel::mpsc::Receiver;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    use super::*;
    use crate::models::server_setting::ServerSetting;

    fn scaffold_app_state() -> (AppState, Receiver<UiCommand>) {
        let manager = Arc::new(RwLock::new(Manager::load()));
        let setting = Arc::new(RwLock::new(ServerSetting::load()));
        let (invoke, receiver) = mpsc::channel(0);
        (
            AppState {
                manager,
                setting,
                invoke,
            },
            receiver,
        )
    }

    #[tokio::test]
    async fn create_job_successfully() {
        let (mut app_state, receiver) = scaffold_app_state();
        let state = Mutex::new(app_state);
        let start = "1";
        let end = "2";
        let version = Version::new(4, 1, 0);
        let path = PathBuf::from("./blender_rs/examples/assets/test.blend".to_owned());
        let output = PathBuf::from("./blender_rs/examples/assets/".to_owned());

        let result = _create_job(start, end, version, path, output, &mut app_state.invoke).await;
        assert!(result.is_ok());

        // make sure to receive AddJobToNetwork event. If this doesn't work then no job will be added across network distribution.
        if let event = receiver.select_next_some().await {
            // how do I compare the enum then?
            assert_eq!(event, UiCommand::AddJobToNetwork(_));
        }
    }

    //#endregion
}
