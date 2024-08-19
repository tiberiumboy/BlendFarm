use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

/*
    Developer blog
    - Initially when I created this class, I thought that this application would
    have a manager of some sort to hold all blender installation installed on this
    machine. It would then save those entry with the server configuration.
        However, recently, I made blender into a separate cargo crate,
    which means I need to migrate some of these code over to the newly created crate
    and let blender crate handle the management of installing, finding version, and govern
    of all blender associated items.
*/

// path to config file name.
const SETTINGS_PATH: &str = "BlendFarm/";
const SETTINGS_FILE_NAME: &str = "ServerSettings.json";
const RENDER_DIR: &str = "RenderData/";

/// Server settings information that the user can load and configure for this program to operate.
/// It will save the list of blender installation on the machine to avoid duplicate download and installation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerSetting {
    pub render_dir: PathBuf,
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

        Self {
            // subject to change - original number came from c# version of BlendFarm
            render_dir: render_data,
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
