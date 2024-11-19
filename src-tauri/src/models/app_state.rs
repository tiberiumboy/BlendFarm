use blender::manager::Manager as BlenderManager;
use blender::models::home::BlenderHome;
use std::sync::{Arc, RwLock};

pub type SafeLock<T> = Arc<RwLock<T>>;

#[derive(Clone)]
pub struct AppState {
    // Had an issue dealing with thread safety over tauri app management state.
    // to keep things simple, we're going to run the network service separately than the main application, and find a way to invoke network command somehow elsewhere.
    // pub network: SafeLock<NetworkService>,
    pub manager: SafeLock<BlenderManager>,
    pub blender_service: SafeLock<BlenderHome>,
}
