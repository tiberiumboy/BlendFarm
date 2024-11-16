use std::sync::{Arc, RwLock};

use crate::services::job_manager::JobManager;

use super::server_setting::ServerSetting;
// use crate::services::server::Server;
use blender::{manager::Manager as BlenderManager, models::home::BlenderHome};

pub type ARW<T> = Arc<RwLock<T>>;

// pub type ServerType = ARW<Server>;
pub type BlenderServiceType = ARW<BlenderHome>;
pub type ServerSettingType = ARW<ServerSetting>;
pub type BlenderManagerType = ARW<BlenderManager>; // would this also fail because it doesn't implement send + sync trait?
pub type JobManagerType = ARW<JobManager>;

#[derive(Clone)]
pub struct AppState {
    // network: ServerType, // I need a network services, but the engine can start delay?
    pub link: BlenderServiceType,
    pub setting: ServerSettingType,
    pub manager: BlenderManagerType,
    pub job_manager: JobManagerType,
}
