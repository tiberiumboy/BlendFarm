use crate::domains::job_store::JobError;
use crate::models::job::{Job, JobAction};
use crate::models::{job::CreatedJobDto, server_setting::ServerSetting};
use crate::services::tauri_app::UiCommand;
use crate::models::setting_action::SettingsAction;
use futures::{channel::mpsc::{self, Sender, SendError}, SinkExt, StreamExt};
use uuid::Uuid;

// TODO: Rename this to AppService instead. This allows us to control background services that creates and process jobs to distribute.
#[derive(Clone)]
pub struct AppState {
    pub invoke: Sender<UiCommand>,
}

impl AppState {

    pub fn new( invoke: Sender<UiCommand> ) -> Self {
        Self {
            invoke
        }
    }

    pub async fn get_settings(&mut self) -> Result<ServerSetting, SendError> {
        let (sender, mut receiver) = mpsc::channel(1);
        let event = UiCommand::Settings(SettingsAction::Get(sender));
        self.invoke.send(event).await?;
        Ok(receiver.select_next_some().await)
    } 

    // functions related to jobs.
    pub async fn create_job(&mut self, job: Job) -> Result<CreatedJobDto, JobError> {
        let (sender, mut receiver) = mpsc::channel(1);
        let add = UiCommand::Job(JobAction::Create(job, sender));
        self
            .invoke
            .send(add)
            .await.map_err(|e| JobError::Send(e.to_string()))?;

        receiver.select_next_some().await
    }

    pub async fn list_jobs(&mut self) -> Result<Option<Vec<CreatedJobDto>>, SendError> {
        let (sender, mut receiver) = mpsc::channel(0);
        let cmd = UiCommand::Job(JobAction::All(sender));
        self.invoke.send(cmd).await?;
        Ok(receiver.select_next_some().await)
    }

    pub async fn fetch_job(&mut self, job_id: Uuid) -> Option<CreatedJobDto> {
        let (sender, mut receiver) = mpsc::channel(0);
        let cmd = UiCommand::Job(JobAction::Find(job_id, sender));
        if let Err(e) = self.invoke.send(cmd).await {
            eprintln!("Fail to send job action: {e:?}");
            return None
        };
        receiver.select_next_some().await
    }
}
