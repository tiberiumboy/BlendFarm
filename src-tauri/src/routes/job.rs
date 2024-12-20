use blender::models::mode::Mode;
use semver::Version;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

use crate::{
    models::{app_state::AppState, job::Job},
    services::tauri_app::UiCommand,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct JobQueue {
    job: Job,
    output: PathBuf,
}

impl JobQueue {
    fn new(job: Job, output: PathBuf ) -> Self {
        Self { job, output }
    }
}

#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    mode: Mode,
    version: Version,
    path: PathBuf,
    output: PathBuf,
) -> Result<JobQueue, Error> {

    let job = Job::new(path, version, mode);
    let mut server = state.lock().await;
    server.jobs.push(job.clone()); 

    // send job to server
    if let Err(e) = server
        .to_network
        .send(UiCommand::StartJob(job.clone()))
        .await
    {
        eprintln!("Fail to send job to the server! \n{e:?}");
    };

    let job_queue = JobQueue::new(job, output);
    Ok(job_queue)
}

/// Abort the job if it's running and delete the entry from the collection list.
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, target_job: Job) -> Result<(), ()> {
    let mut server = state.lock().await; // Should I worry about posion error?
    server.jobs.retain(|x| x.eq(&target_job));
    let msg = UiCommand::StopJob(target_job.as_ref().clone());
    if let Err(e) = server.to_network.send(msg).await {
        eprintln!("Fail to send stop job command! {e:?}");
    }
    Ok(())
}

/// List all available jobs stored in the collection.
#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let data = serde_json::to_string(&server.jobs).unwrap();
    Ok(data)
}
