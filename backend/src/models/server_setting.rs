use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

use blender::blender::Blender;

const SETTINGS_PATH: &str = "./ServerSettings.json";
const BLENDER_DATA: &str = "BlenderData";
const RENDER_DATA: &str = "RenderData";
// const BLENDER_FILES: &str = "BlenderFiles";

/// Server settings information that the user can load and configure for this program to operate.
/// It will save the list of blender installation on the machine to avoid duplicate download and installation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSetting {
    pub port: u16,
    pub broadcast_port: u16,
    pub blender_data: PathBuf,
    pub render_data: PathBuf,
    pub blenders: Vec<Blender>, // list of installed blender versions on this machine.
}

// pub trait TempDirectory {
//     fn get_tmp_dir() -> PathBuf;
//     // TODO find a way to implement generic function that can be shared across all other directory like structure.
// }

fn create_tmp_dir(dir_name: &str) -> PathBuf {
    let mut tmp = std::env::temp_dir();
    tmp.push(dir_name);
    if !tmp.exists() {
        fs::create_dir(&tmp).expect("Unable to create directory! Permission issue?");
    }
    tmp
}

impl Default for ServerSetting {
    fn default() -> Self {
        Self {
            port: 15000,
            broadcast_port: 16342,
            blender_data: create_tmp_dir(BLENDER_DATA),
            render_data: create_tmp_dir(RENDER_DATA),
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
        match fs::read_to_string(SETTINGS_PATH) {
            // TODO: find a way to handle parsing the error?
            Ok(data) => serde_json::from_str(&data).expect("Unable to parse settings!"),
            Err(_) => {
                let data = ServerSetting::default();
                let _ = &data.save();
                data
            }
        }
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
