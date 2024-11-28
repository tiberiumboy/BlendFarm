use blender::models::mode::Mode;
use semver::Version;
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

use crate::{
    models::{app_state::AppState, job::Job, project_file::ProjectFile},
    services::network_service::Command,
};

// FIgure out why I can't get this to work?
#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    file_path: PathBuf,
    output: PathBuf,
    version: Version,
    mode: Mode,
    // info: CreateJobRequest, // this code is complaining that it's missing required key info?
) -> Result<Job, Error> {
    let file_path = file_path;
    let output = output;
    let project_file =
        ProjectFile::new(file_path).map_err(|e| Error::AssetNotFound(e.to_string()))?;
    let job = Job::new(project_file, output, version, mode);
    let mut server = state.lock().await;
    server.jobs.push(job.clone());

    // send job to server
    if let Err(e) = server
        .to_network
        .send(Command::StartJob(job.clone()))
        .await
    {
        println!("Fail to send job to the server! \n{e:?}");
    }

    // TODO: Impl a way to send the files to the rendering nodes.
    // _server.send_job(job.clone());
    Ok(job)
}

/// Abort the job if it's running and delete the entry from the collection list.
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, target_job: Job) -> Result<(), ()> {
    let mut server = state.lock().await; // Should I worry about posion error?
    server.jobs.retain(|x| x.eq(&target_job));
    Ok(())
}

/// List all available jobs stored in the collection.
// this function invoked twice?
// it may seems like the component is re-rendered twice. Check there first.
#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let jobs = server.jobs.clone();
    let data = serde_json::to_string(&jobs).unwrap();
    Ok(data)
}
