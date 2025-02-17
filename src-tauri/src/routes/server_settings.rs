use tauri::{command, State};
// TODO: Double verify that this is the correct Mutex usage throughout the application
use tokio::sync::Mutex;

use crate::models::{app_state::AppState, server_setting::ServerSetting};


#[command(async)]
pub async fn get_server_settings() -> Result<String, String> {
    Ok( "".to_owned() )
}

#[command(async)]
pub async fn set_server_settings(
    state: State<'_, Mutex<AppState>>,
    new_settings: ServerSetting,
) -> Result<(), String> {
    // maybe I'm a bit confused here?
    let app_state = state.lock().await;
    let mut old_setting = app_state.setting.write().await;
    new_settings.save();
    *old_setting = new_settings;

    Ok(())
}