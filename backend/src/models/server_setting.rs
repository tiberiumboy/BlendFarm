use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

use blender::blender::Blender;

const SETTINGS_PATH: &str = "ServerSettings";
const BLENDER_DATA: &str = "BlenderData";
const RENDER_DATA: &str = "RenderData";
// const BLENDER_FILES: &str = "BlenderFiles";

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSetting {
    pub port: u16,
    pub broadcast_port: u16,
    pub blender_data: BlenderData,
    pub render_data: RenderData,
    // TODO: Find out how rust program load and read configurations and compare before running a new blender job.
    pub blenders: Vec<Blender>, // list of installed blender versions on this machine.
}

// pub trait TempDirectory {
//     fn get_tmp_dir() -> PathBuf;
//     // TODO find a way to implement generic function that can be shared across all other directory like structure.
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderData {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderData {
    pub path: PathBuf,
}

fn create_tmp_dir(dir_name: &str) -> PathBuf {
    let mut tmp = std::env::temp_dir();
    tmp.push(dir_name);
    if !tmp.exists() {
        fs::create_dir(&tmp).expect("Unable to create directory!");
    }
    tmp
}

impl Default for BlenderData {
    fn default() -> Self {
        Self {
            path: create_tmp_dir(BLENDER_DATA),
        }
    }
}

impl Default for RenderData {
    fn default() -> Self {
        Self {
            path: create_tmp_dir(RENDER_DATA),
        }
    }
}

impl Default for ServerSetting {
    fn default() -> Self {
        Self {
            port: 15000,
            broadcast_port: 16342,
            blender_data: BlenderData::default(),
            render_data: RenderData::default(),
            blenders: vec![],
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
