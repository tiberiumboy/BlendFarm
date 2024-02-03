use std::fs;

use serde::{Deserialize, Serialize};

const BLENDER_DATA: &str = "BlenderData";
const RENDER_DATA: &str = "RenderData";
const SETTINGS_PATH: &str = "ServerSettings";
const BLENDER_FILES: &str = "BlenderFiles";

#[derive(Serialize, Deserialize)]
pub struct ServerSettings {
    pub port: u16,
    pub broadcast_port: u16,
    pub blender_data: String,
    pub render_data: String,
    pub blender_files: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 15000,
            broadcast_port: 16342,
            blender_data: BLENDER_DATA.to_owned(),
            render_data: RENDER_DATA.to_owned(),
            blender_files: BLENDER_FILES.to_owned(),
        }
    }
}

impl ServerSettings {
    pub fn save(&self) {
        // save this data to...?
        let data = serde_json::to_string(&self).expect("Unable to parse ServerSettings into json!");
        fs::write(SETTINGS_PATH, data).expect("Unable to write file! Permission issue?");
    }

    pub fn load() -> ServerSettings {
        // load server settings from config?
        let path = SETTINGS_PATH; // SystemInfo.RelativeToApplicationDirectory(SETTINGS_PATH)???
        let data = fs::read_to_string(path).expect("Unable to read file!");
        let server_settings: ServerSettings =
            serde_json::from_str(&data).expect("Unable to parse settings!");
        server_settings
    }
}
