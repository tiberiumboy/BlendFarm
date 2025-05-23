use super::server_setting::ServerSetting;
use crate::services::tauri_app::UiCommand;
use blender::manager::Manager as BlenderManager;
use futures::channel::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SafeLock<T> = Arc<RwLock<T>>;

#[derive(Clone)]
pub struct AppState {
    pub manager: SafeLock<BlenderManager>,
    pub setting: SafeLock<ServerSetting>,
    pub invoke: Sender<UiCommand>,
}
