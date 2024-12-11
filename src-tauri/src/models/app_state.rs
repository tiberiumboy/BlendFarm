use super::{job::Job, server_setting::ServerSetting};
use crate::services::tauri_app::UiCommand;
use blender::manager::Manager as BlenderManager;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

pub type SafeLock<T> = Arc<RwLock<T>>;

#[derive(Clone)]
pub struct AppState {
    pub manager: SafeLock<BlenderManager>,
    pub to_network: Sender<UiCommand>,
    pub setting: SafeLock<ServerSetting>,
    pub jobs: Vec<Job>,
}
