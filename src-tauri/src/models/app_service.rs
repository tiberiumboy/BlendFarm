use crate::models::job::{Job, JobAction};
use crate::models::setting_action::SettingsAction;
use crate::models::{job::CreatedJobDto, server_setting::ServerSetting};
use crate::services::tauri_app::UiCommand;
use anyhow::Error;
use blender_rs::manager::Manager as BlenderManager;
use futures::channel::oneshot;
use futures::{
    SinkExt,
    channel::mpsc::{SendError, Sender},
};
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

// TODO: Rename this to AppService instead. This allows us to control background services that creates and process jobs to distribute.
#[derive(Clone)]
pub struct AppService {
    invoke: Sender<UiCommand>,
    blender_manager: Arc<Mutex<BlenderManager>>,
}

impl AppService {
    pub fn new(invoke: Sender<UiCommand>, blender_manager: Arc<Mutex<BlenderManager>>) -> Self {
        Self {
            invoke,
            blender_manager,
        }
    }

    pub fn send(&mut self, message: UiCommand) {
        self.invoke.send(message);
    }

    pub async fn get_settings(&mut self) -> Result<ServerSetting, SendError> {
        let (sender, receiver) = futures::channel::oneshot::channel::<ServerSetting>();
        let event = UiCommand::Settings(SettingsAction::Get(sender));
        self.invoke.send(event).await?;
        Ok(receiver
            .await
            .expect("TODO: See how this could break and resolve this?"))
    }

    // functions related to jobs.
    pub async fn create_job(&mut self, job: Job) -> Result<CreatedJobDto, Error> {
        let (sender, receiver) = oneshot::channel();
        let add = UiCommand::Job(JobAction::Create(job, sender));
        self.invoke.send(add).await?;
        let result = receiver.await?;
        Ok(result)
    }

    pub async fn list_jobs(&mut self) -> Result<Vec<CreatedJobDto>, Error> {
        let (sender, receiver) = oneshot::channel::<Vec<CreatedJobDto>>();
        let cmd = UiCommand::Job(JobAction::All(sender));
        self.invoke.send(cmd).await;
        Ok(receiver.await?)
    }

    pub async fn fetch_job(&mut self, job_id: Uuid) -> Option<CreatedJobDto> {
        let (sender, receiver) = oneshot::channel::<Option<CreatedJobDto>>();
        let cmd = UiCommand::Job(JobAction::Find(job_id, sender));
        if let Err(e) = self.invoke.send(cmd).await {
            eprintln!("Fail to send job action: {e:?}");
            return None;
        };
        receiver.await.expect("Should be infallible")
    }
}
