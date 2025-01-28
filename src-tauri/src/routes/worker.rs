use tauri::{command, State};
use tokio::sync::Mutex;

use crate::models::app_state::AppState;

#[command(async)]
pub async fn list_workers(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let workers = server.worker_db.read().await;
    match &workers.list_worker().await {
        Ok(data) => Ok(serde_json::to_string(data).unwrap()),
        Err(e) => Err(e.to_string()),
    }
}
