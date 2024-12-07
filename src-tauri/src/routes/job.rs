use blender::models::mode::Mode;
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

use crate::{
    models::{app_state::AppState, job::Job, project_file::ProjectFile},
    UiCommand,
};

#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    project_file: ProjectFile,
    output: PathBuf,
    mode: Mode,
) -> Result<Job, Error> {
    let path = project_file.path.clone();
    let job = Job::new(project_file, output, mode);
    let mut server = state.lock().await;
    server.jobs.push(job.clone());

    // upload the file to file share services
    let msg = UiCommand::UploadFile(path);
    if let Err(e) = server.to_network.send(msg).await {
        eprintln!("Fail to upload file! {e:?}");
    }

    // send job to server
    if let Err(e) = server
        .to_network
        .send(UiCommand::StartJob(job.clone()))
        .await
    {
        eprintln!("Fail to send job to the server! \n{e:?}");
    };

    Ok(job)
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
    let jobs = server.jobs.clone();
    let data = serde_json::to_string(&jobs).unwrap();
    Ok(data)
}
