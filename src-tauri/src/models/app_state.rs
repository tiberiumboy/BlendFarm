use crate::{models::server_setting::ServerSetting, services::tauri_app::{SettingsAction, UiCommand}};
use futures::{channel::mpsc::{self, Sender}, SinkExt, StreamExt};

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
    
    pub async fn get_settings(&mut self) -> Result<ServerSetting, mpsc::SendError> {
        let (sender, mut receiver) = mpsc::channel(1);
        let event = UiCommand::Settings(SettingsAction::Get(sender));
        self.invoke.send(event).await?;
        Ok(receiver.select_next_some().await)
    } 
}
