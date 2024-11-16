use std::sync::{Arc, RwLock};
use crate::services::network_service::NetworkService;
use super::{job::Job, server_setting::ServerSetting};
use blender::{manager::Manager as BlenderManager, models::home::BlenderHome};

pub type SafeLock<T> = Arc<RwLock<T>>;

pub type NetworkServiceType = SafeLock<NetworkService>;
pub type BlenderServiceType = SafeLock<BlenderHome>;
pub type ServerSettingType = SafeLock<ServerSetting>;
pub type BlenderManagerType = SafeLock<BlenderManager>; // would this also fail because it doesn't implement send + sync trait?

#[derive(Clone)]
pub struct AppState {
    pub network: NetworkServiceType, // I need a network services, but the engine can start delay?
    pub link: BlenderServiceType,
    pub setting: ServerSettingType,
    pub manager: BlenderManagerType,
    pub jobs: SafeLock<Vec<Job>>,
}
