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
) -> Result<Job, ()> {
    let project_file =
        ProjectFile::new(info.file_path).map_err(|e| Error::AssetNotFound(e.to_string()))?;
    let job = Job::new(project_file, info.output, info.version, info.mode);
    let server = state.lock().unwrap();

    // send job to server
    let mut manager = server.job_manager.write().unwrap();
    let _ = manager.add_to_queue(job.clone()).await.map_err(|_| ())?;
    // TODO: Impl a way to send the files to the rendering nodes.
    // server.send_job(job.clone());
    Ok(job)
}

/// Abort the job if it's running and delete the entry from the collection list.
///
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, target_job: Job) -> Result<(), ()> {
    let data = state.lock().unwrap();
    let mut manager = data
        .job_manager
        .write()
        .expect("Unable to obtain job manager");

    match manager.remove_from_queue(target_job.as_ref()).await {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

/// List all available jobs stored in the collection.
#[command]
pub fn list_jobs(_state: State<AppState>) -> Result<String, Error> {
    // let server = state.lock().unwrap();
    // let data = server.get_job_list();
    // let data = serde_json::to_string(&data).unwrap();
    Ok("data".to_owned())
}
