// Used to help organize dependency injections
use blender_rs::manager::Manager as BlenderManager;
use crate::models::server_setting::ServerSetting;

pub(crate) struct AppContext {
    pub manager: BlenderManager,
    pub settings: ServerSetting, // default ::load()
}

impl AppContext {
    pub fn new(manager: BlenderManager) -> Self {
        let settings = ServerSetting::load();
        Self { manager, settings }
    }
}
