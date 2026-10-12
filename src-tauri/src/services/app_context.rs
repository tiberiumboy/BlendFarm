// Used to help organize dependency injections
use crate::models::server_setting::ServerSetting;
use blender_rs::manager::Manager as BlenderManager;

pub(crate) struct AppContext {
    pub manager: BlenderManager,
    pub settings: ServerSetting,
}

impl AppContext {
    pub fn new(manager: BlenderManager) -> Self {
        let settings = ServerSetting::load();
        Self { manager, settings }
    }
}
