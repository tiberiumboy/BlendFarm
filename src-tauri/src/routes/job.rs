use blender::models::mode::Mode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use tauri::{command, Error, State};

use crate::models::{app_state::AppState, job::Job, project_file::ProjectFile};

#[derive(Serialize, Deserialize)]
pub struct CreateJobRequest {
    file_path: PathBuf,
    output: PathBuf,
    version: Version,
    mode: Mode,
}

#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    info: CreateJobRequest,
) -> Result<Job, Error> {
    let project_file =
        ProjectFile::new(info.file_path).map_err(|e| Error::AssetNotFound(e.to_string()))?;
    let job = Job::new(project_file, info.output, info.version, info.mode);
    let _server = state.lock().unwrap();

    // send job to server

    // let mut manager = _server.jobs.write().unwrap();
    // let _ = manager
    //     .add_to_queue(job.clone())
    //     .map_err(|e| Error::AssetNotFound(e.to_string()))?;

    // TODO: Impl a way to send the files to the rendering nodes.
    // _server.send_job(job.clone());
    Ok(job)
}

/// Abort the job if it's running and delete the entry from the collection list.
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, _target_job: Job) -> Result<(), ()> {
    let _data = state.lock().unwrap();
    // let mut manager = data
    //     .jobs
    //     .write()
    //     .expect("Unable to obtain job manager");

    // let _ = manager.remove_from_queue(target_job.as_ref());
    Ok(())
}

/// List all available jobs stored in the collection.
#[command(async)]
pub async fn list_jobs(state: State<'_,Mutex<AppState>>) -> Result<String, String> {
    let _server = state.lock().unwrap();
    // let manager = server.jobs.read().unwrap();
    // let data = serde_json::to_string(manager.as_ref()).unwrap();
    // Ok(data)
    Ok("".to_owned())
}
