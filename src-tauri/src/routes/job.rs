use blender::models::mode::Mode;
use build_html::{Html, HtmlElement, HtmlTag};
use semver::Version;
use std::ops::Range;
use std::path::PathBuf;
use tauri::{command, State};
use tokio::sync::Mutex;

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
) -> Result<String, String> {
    // first thing first, parse the string into number
    let start= start.parse::<i32>().map_err(|e| e.to_string())?;
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

    Ok("".to_owned())
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

#[command]
pub fn job_detail() -> String {
    // TODO: ask for the key to fetch the job details.

    HtmlElement::new(HtmlTag::Div)
        .with_child(
            HtmlElement::new(HtmlTag::ParagraphText)
                .with_child("Job Detail".into())
                .into(),
        )
        .to_html_string()
}