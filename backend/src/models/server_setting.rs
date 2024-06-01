use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use blender::blender::Blender;

// path to config file name.
const SETTINGS_PATH: &str = "./ServerSettings.json";
// Blender data needs to be saved in the user's document settings to retain and save blender location.
// TODO: Find a way to fetch user's directory and use that directory for Blender_data const variable
const BLENDER_DATA: &str = "BlenderData/";
// RenderData can be used in a temp directory because I do not expect this to be long lived.
const RENDER_DATA: &str = "RenderData/";
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

impl Default for ServerSetting {
    fn default() -> Self {
        let mut render_data = std::env::temp_dir();
        render_data.push(RENDER_DATA);
        fs::create_dir_all(&render_data).unwrap();

        let blender_data = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(BLENDER_DATA);
        fs::create_dir_all(&blender_data).unwrap();
        Self {
            port: 15000,
            broadcast_port: 16342,
            blender_data,
            render_data,
            blenders: vec![],
        }
    }
}

impl ServerSetting {
    pub fn save(&self) {
        // save this data to...?
        let data = serde_json::to_string(&self).expect("Unable to parse ServerSettings into json!");
        fs::write(SETTINGS_PATH, data).expect("Unable to write file! Permission issue?");
    }

    // TODO: Consider about returning Result<ServerSetting> or Result<Self>?
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
}
