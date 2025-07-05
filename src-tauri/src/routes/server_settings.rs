use crate::{
    models::{app_state::AppState, server_setting::ServerSetting},
    services::tauri_app::{SettingsAction, UiCommand},
};
use futures::SinkExt;
use tauri::{State, command};
use tokio::sync::Mutex;

#[command(async)]
pub async fn get_server_settings() -> Result<String, String> {
    Ok("".to_owned())
}

#[command(async)]
pub async fn set_server_settings(
    state: State<'_, Mutex<AppState>>,
    new_settings: ServerSetting,
) -> Result<(), String> {
    let mut app_state = state.lock().await;
    let event = UiCommand::Settings(SettingsAction::Update(new_settings));
    if let Err(e) = app_state.invoke.send(event).await {
        return Err(e.to_string());
    }
    Ok(())
}
