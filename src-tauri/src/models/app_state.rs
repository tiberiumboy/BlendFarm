use super::{job::Job, server_setting::ServerSetting};
use crate::services::network_service::Command;
use blender::manager::Manager as BlenderManager;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Sender;

pub type SafeLock<T> = Arc<RwLock<T>>;

#[derive(Clone)]
pub struct AppState {
    // Had an issue dealing with thread safety over tauri app management state.
    // to keep things simple, we're going to run the network service separately than the main application, and find a way to invoke network command somehow elsewhere.
    // pub network: SafeLock<NetworkService>,
    pub manager: SafeLock<BlenderManager>,
    pub to_network: Sender<Command>,
    pub setting: SafeLock<ServerSetting>,
    pub jobs: Vec<Job>,
}
