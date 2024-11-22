use blender::models::mode::Mode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use tauri::{command, Error, State};

use crate::models::{app_state::AppState, job::Job, project_file::ProjectFile};

// TODO: currently the program will report that it's missing key info for this struct. Figure out why?
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    file_path: PathBuf,
    output: PathBuf,
    version: Version,
    // mode: Mode,
}

// FIgure out why I can't get this to work?
#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    info: CreateJobRequest, // this code is complaining that it's missing required key info?
) -> Result<Job, Error> {
    let file_path = info.file_path;
    let output = info.output;
    let mode = Mode::Frame(1);

    let project_file =
        ProjectFile::new(file_path).map_err(|e| Error::AssetNotFound(e.to_string()))?;
    let job = Job::new(project_file, output, info.version, mode);
    let mut server = state.lock().unwrap();
    server.jobs.push(job.clone());

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
pub async fn delete_job(state: State<'_, Mutex<AppState>>, target_job: Job) -> Result<(), ()> {
    let mut server = state.lock().unwrap(); // Should I worry about posion error?
    server.jobs.retain(|x| x.eq(&target_job));
    Ok(())
}

/// List all available jobs stored in the collection.
#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().unwrap();
    let jobs = server.jobs.clone();
    let data = serde_json::to_string(&jobs).unwrap();
    Ok(data)
}
