use crate::models::{
    app_service::AppService, server_setting::ServerSetting, setting_action::SettingsAction,
};
use crate::services::tauri_app::UiCommand;
// use futures::SinkExt;
use tauri::{State, command};
use tokio::sync::Mutex;

#[command(async)]
pub async fn get_server_settings() -> Result<String, String> {
    Ok("".to_owned())
}

#[command(async)]
pub async fn set_server_settings(
    state: State<'_, Mutex<AppService>>,
    new_settings: ServerSetting,
) -> Result<(), String> {
    let mut app_state = state.lock().await;
    let event = UiCommand::Settings(SettingsAction::Update(new_settings));
    app_state.send(event);
    Ok(())
}
