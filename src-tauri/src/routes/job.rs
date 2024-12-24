use blender::models::mode::Mode;
use semver::Version;
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

use crate::{
    models::{app_state::AppState, job::Job},
    services::tauri_app::UiCommand,
};

#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    mode: Mode,
    version: Version,
    path: PathBuf,
    output: PathBuf,
) -> Result<Job, Error> {
    // this is definitely a hack and should probably handle unwrap functions()
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    let job = Job::new(path, output, version, mode);
    let server = state.lock().await;
    let mut jobs = server.job_db.write().await;
    // use this to send the job over to database instead of command to network directly.
    // We're splitting this apart to rely on database collection instead of forcing to send command over.
    let _ = jobs.add_job(job.clone());
    
    // send job to server
    if let Err(e) = server
        .to_network
        .send(UiCommand::UploadFile(job.get_project_path().clone(), file_name))
        // .send(UiCommand::StartJob(job.clone()))
        .await
    {
        eprintln!("Fail to send job to the server! \n{e:?}");
    };

    Ok(job)
}

/// just delete the job from database. Notify peers to abandon task matches job_id
#[command(async)]
pub async fn delete_job(state: State<'_, Mutex<AppState>>, target_job: Job) -> Result<(), ()> {
    let server = state.lock().await; // Should I worry about posion error?
    let mut jobs = server.job_db.write().await;
    let id = target_job.as_ref();
    let _ = jobs.delete_job(id.clone());
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
    let jobs = server.job_db.read().await;
    match &jobs.list_all().await {
        Ok(data) => Ok(serde_json::to_string(data).unwrap()),
        Err(e) => Err(e.to_string()),
    }
}
