use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

const BLENDER_DATA: &str = "BlenderData";
const RENDER_DATA: &str = "RenderData";
const SETTINGS_PATH: &str = "ServerSettings";
const BLENDER_FILES: &str = "BlenderFiles";

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSetting {
    pub port: u16,
    pub broadcast_port: u16,
    pub blender_data: String,
    pub render_data: String,
    pub blender_files: String,
}

impl Default for ServerSetting {
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

impl ServerSetting {
    pub fn save(&self) {
        // save this data to...?
        let data = serde_json::to_string(&self).expect("Unable to parse ServerSettings into json!");
        fs::write(SETTINGS_PATH, data).expect("Unable to write file! Permission issue?");
    }

    pub fn load() -> ServerSetting {
        // load server settings from config?
        let path = SETTINGS_PATH; // SystemInfo.RelativeToApplicationDirectory(SETTINGS_PATH)???
        let data = fs::read_to_string(path).expect("Unable to read file!");
        let server_settings: ServerSetting =
            serde_json::from_str(&data).expect("Unable to parse settings!");
        server_settings
    }

    pub fn get_blender_data() -> io::Result<PathBuf> {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("/{}/", BLENDER_DATA));
        std::fs::create_dir_all(&tmp)?;
        Ok(tmp)
    }

    pub fn get_render_data() -> io::Result<PathBuf> {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("/{}/", RENDER_DATA));
        std::fs::create_dir(&tmp)?;
        Ok(tmp)
    }
}
