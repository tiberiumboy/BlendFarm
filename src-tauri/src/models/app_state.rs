use super::server_setting::ServerSetting;
use crate::domains::{job_store::JobStore, worker_store::WorkerStore};
use crate::services::tauri_app::UiCommand;
use blender::manager::Manager as BlenderManager;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc::Sender};

pub type SafeLock<T> = Arc<RwLock<T>>;

// wonder if this is required?
// #[derive(Clone)]
pub struct AppState {
    pub manager: SafeLock<BlenderManager>,
    pub to_network: Sender<UiCommand>,
    pub setting: SafeLock<ServerSetting>,
    pub job_db: SafeLock<(dyn JobStore + Send + Sync + 'static)>,
    pub worker_db: SafeLock<(dyn WorkerStore + Send + Sync + 'static)>,
}
