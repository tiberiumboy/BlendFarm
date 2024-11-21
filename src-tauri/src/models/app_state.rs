use blender::manager::Manager as BlenderManager;
use blender::models::home::BlenderHome;
use std::sync::{Arc, RwLock};

use super::{job::Job, server_setting::ServerSetting};

pub type SafeLock<T> = Arc<RwLock<T>>;

#[derive(Clone)]
pub struct AppState {
    // Had an issue dealing with thread safety over tauri app management state.
    // to keep things simple, we're going to run the network service separately than the main application, and find a way to invoke network command somehow elsewhere.
    // pub network: SafeLock<NetworkService>,
    pub manager: SafeLock<BlenderManager>,
    pub blender_source: SafeLock<BlenderHome>,
    pub setting: SafeLock<ServerSetting>,
    pub jobs: Vec<Job>,
}
