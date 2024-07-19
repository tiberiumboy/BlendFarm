use blender::blender::{Blender, BlenderJSON};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

// path to config file name.
const SETTINGS_PATH: &str = "BlendFarm/";
const SETTINGS_FILE_NAME: &str = "ServerSettings.json";
const BLENDER_DIR: &str = "BlenderData/";
const RENDER_DIR: &str = "RenderData/";
// const BLENDER_FILES: &str = "BlenderFiles";

/// Server settings information that the user can load and configure for this program to operate.
/// It will save the list of blender installation on the machine to avoid duplicate download and installation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSetting {
    pub port: u16,
    pub blender_dir: PathBuf,
    pub render_dir: PathBuf,
    pub blenders: Vec<BlenderJSON>, // list of installed blender versions on this machine.
}

impl Default for ServerSetting {
    fn default() -> Self {
        // This can be subject to change. Currently had it set to temporary directory
        // due to the fact that we do not want to store image on the computer once we
        // successfully transfer to the host machine. It would be used as backup archive
        // in case the host machine went abruptly. (Maybe a feature?)
        let mut render_data = std::env::temp_dir();
        render_data.push(RENDER_DIR);
        // ensure path exists
        fs::create_dir_all(&render_data).unwrap();

        // Setting blender installation to user's download directory.
        let blender_data = dirs::download_dir().unwrap().join(BLENDER_DIR);
        // ensure path exists
        fs::create_dir_all(&blender_data).unwrap();

        Self {
            // subject to change - original number came from c# version of BlendFarm
            port: 15000,
            blender_dir: blender_data,
            render_dir: render_data,
            blenders: vec![],
        }
    }
}

impl ServerSetting {
    fn get_config_path() -> PathBuf {
        let path = dirs::config_dir().unwrap().join(SETTINGS_PATH);
        fs::create_dir_all(&path).expect("Unable to create directory!");
        path.join(SETTINGS_FILE_NAME)
    }

    /// Save the configurations to the user's config directory.
    pub fn save(&self) {
        let data = serde_json::to_string(&self).expect("Unable to parse ServerSettings into json!");
        let config_path = Self::get_config_path();
        fs::write(config_path, data).expect("Unable to write file! Permission issue?");
    }

    pub fn get_latest_blender(&mut self) -> Blender {
        match self.blenders.
    }

    pub fn get_blender(&mut self, version: Version) -> Blender {
        // eventually we would want to check if the version is already installed on the machine.
        // otherwise download and install the version prior to run this script.
        match self.blenders.iter().find(|&x| x.version == version) {
            Some(json) => Blender::from_json(json.to_owned()).unwrap(),
            None => {
                let blender = Blender::download(version, &self.blender_dir).unwrap();
                self.blenders.push(blender.get_serialized_data());
                self.save();
                blender
            }
        }
    }

    /// Load user configurations from the user's config directory
    pub fn load() -> ServerSetting {
        let config_path = Self::get_config_path();
        match fs::read_to_string(config_path) {
            Ok(data) => serde_json::from_str(&data).expect("Unable to parse settings!"),
            Err(_) => {
                let data = ServerSetting::default();
                let _ = &data.save();
                data
            }
        }
    }
}
