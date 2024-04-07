use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

const BLENDER_DATA: &str = "BlenderData";
const RENDER_DATA: &str = "RenderData";
const SETTINGS_PATH: &str = "ServerSettings";
const BLENDER_FILES: &str = "BlenderFiles";

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSetting {
    pub port: u16,
    pub broadcast_port: u16,
    pub blender_data: PathBuf,
    pub render_data: PathBuf,
    pub blender_files: PathBuf,
}

fn create_tmp_dir(name: &str) -> PathBuf {
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("/{}/", BLENDER_DATA));
    std::fs::create_dir(&tmp).expect("Unable to create directory!");
    tmp
}

impl Default for ServerSetting {
    fn default() -> Self {
        // repeated code here :(
        let blend_data = create_tmp_dir(BLENDER_DATA);
        let blend_files = create_tmp_dir(BLENDER_FILES);
        let render_data = create_tmp_dir(RENDER_DATA);

        Self {
            port: 15000,
            broadcast_port: 16342,
            blender_data: PathBuf::from(blend_data),
            render_data: PathBuf::from(render_data),
            blender_files: PathBuf::from(blend_files),
        }
    }
}

#[allow(dead_code)]
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
