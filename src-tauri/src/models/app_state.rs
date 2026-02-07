use crate::models::server_setting::ServerSetting;
use crate::services::tauri_app::UiCommand;
use crate::models::setting_action::SettingsAction;
use futures::{channel::mpsc::{self, Sender, SendError}, SinkExt, StreamExt};

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
}
