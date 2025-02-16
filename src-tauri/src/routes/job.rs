use blender::models::mode::Mode;
use maud::html;
use semver::Version;
use std::path::PathBuf;
use std::{ops::Range, str::FromStr};
use tauri::{command, State};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    models::{app_state::AppState, job::Job},
    services::tauri_app::UiCommand,
};

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
) -> Result<(), String> {
    // first thing first, parse the string into number
    let start = start.parse::<i32>().map_err(|e| e.to_string())?;
    let end = end.parse::<i32>().map_err(|e| e.to_string())?;
    // stop if the parse fail to parse.

    let mode = Mode::Animation(Range { start, end });
    let job = Job::from(path, output, version, mode);
    let server = state.lock().await;
    let mut jobs = server.job_db.write().await;

    // use this to send the job over to database instead of command to network directly.
    // We're splitting this apart to rely on database collection instead of forcing to send command over.
    if let Err(e) = jobs.add_job(job.clone()).await {
        eprintln!("{:?}", e);
    }

    // send job to server
    if let Err(e) = server
        .to_network
        .send(UiCommand::StartJob(job.clone()))
        .await
    {
        eprintln!("Fail to send command to the server! \n{e:?}");
    }

    Ok(())
}

#[command(async)]
pub async fn list_jobs(state: State<'_, Mutex<AppState>>) -> Result<String, ()> {
    let server = state.lock().await;
    let jobs = server.job_db.read().await;
    let job_list = jobs.list_all().await.unwrap();

    Ok(html! {
        @for job in job_list {
            div {
                table {
                    tbody {
                        tr tauri-invoke="get_job" hx-include="[name='jobId']" hx-target="#detail" {
                            input type="hidden" name="jobId" value=(job.id.to_string());
                            td style="width:100%" {
                                (job.get_file_name())
                            };
                        };
                    };
                };
            };
        };
    }
    .0)
}

#[command(async)]
pub async fn get_job(state: State<'_, Mutex<AppState>>, job_id: &str) -> Result<String, ()> {
    // TODO: ask for the key to fetch the job details.
    let job_id = Uuid::from_str(job_id).map_err(|e| {
        eprintln!("Unable to parse uuid? \n{e:?}");
        ()
    })?;
    let app_state = state.lock().await;
    let jobs = app_state.job_db.read().await;

    match jobs.get_job(&job_id).await {
        Ok(job) => Ok(html!(
        div {
                p { "Job Detail" };
                div { ( job.project_file.to_str().unwrap() ) };
                div { ( job.output.to_str().unwrap() ) };
                // div { ( job.mode ) };
                div { ( job.blender_version.to_string() ) };
            };
        )
        .0),
        Err(e) => Ok(html!(
        div {
                p { "Job do not exist.. How did you get here?" };
                input type="hidden" value=(e.to_string());
            };
        )
        .0),
    }
}

// we'll need to figure out more about this? How exactly are we going to update the job?
// #[command(async)]
// pub fn update_job()

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
