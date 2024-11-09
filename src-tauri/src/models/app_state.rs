use std::sync::{Arc, RwLock};

use crate::domain::job_manager::JobManager;

use super::server_setting::ServerSetting;
// use crate::services::server::Server;
use blender::{manager::Manager as BlenderManager, models::download_link::BlenderHome};

pub type ARW<T: Sync + Send> = Arc<RwLock<T>>;

// pub type ServerType = ARW<Server>;
pub type BlenderServiceType = ARW<BlenderHome>;
pub type ServerSettingType = ARW<ServerSetting>;
pub type BlenderManagerType = ARW<BlenderManager>;
pub type JobManagerType = ARW<dyn JobManager>;

#[derive(Clone)]
pub struct AppState {
    // network: ServerType,
    pub link: BlenderServiceType,
    pub setting: ServerSettingType,
    pub manager: BlenderManagerType,
    pub job_manager: JobManagerType,
}
