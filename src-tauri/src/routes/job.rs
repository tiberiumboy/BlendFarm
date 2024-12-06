use blender::models::mode::Mode;
use futures::channel::oneshot;
use std::path::PathBuf;
use tauri::{command, Error, State};
use tokio::sync::Mutex;

use crate::models::{
    app_state::AppState, job::Job, message::NetCommand, project_file::ProjectFile,
};

// Figure out why I can't get this to work?
#[command(async)]
pub async fn create_job(
    state: State<'_, Mutex<AppState>>,
    project_file: ProjectFile,
    output: PathBuf,
    mode: Mode,
) -> Result<Job, Error> {
    let file_name = project_file.get_file_name().to_string();
    let job = Job::new(project_file, output, mode);
    let mut server = state.lock().await;
    server.jobs.push(job.clone());

    let (sender, receiver) = oneshot::channel();

    // upload the file to file share services
    if let Err(e) = server
        .to_network
        .send(NetCommand::StartProviding { file_name, sender })
        .await
    {
        eprintln!("Fail to upload file! {e:?}");
    }
    receiver.await.expect("Sender should not be dropped!");

    // send job to server
    if let Err(e) = server
        .to_network
        .send(NetCommand::StartJob(job.clone()))
        .await
    {
        println!("Fail to send job to the server! \n{e:?}");
    };

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
#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let jobs = server.jobs.clone();
    let data = serde_json::to_string(&jobs).unwrap();
    Ok(data)
}
