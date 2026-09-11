use serde::{Deserialize, Serialize};
use std::io::Result as IoResult;
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
const BLEND_DIR: &str = "BlendFiles/";

/// Server settings information that the user can load and configure for this program to operate.
/// It will save the list of blender installation on the machine to avoid duplicate download and installation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerSetting {
    /// Public directory to store all finished render image.
    pub render_dir: PathBuf,
    /// Public directory of blender working copy of files.
    pub blend_dir: PathBuf,
}

impl Default for ServerSetting {
    fn default() -> Self {
        // This can be subject to change. Currently had it set to temporary directory
        // due to the fact that we do not want to store image on the computer once we
        // successfully transfer to the host machine. It would be used as backup archive
        // in case the host machine went abruptly. (Maybe a feature?)
        let base_path = std::env::temp_dir();
        let mut render_dir = base_path.clone();
        render_dir.push(RENDER_DIR);

        let mut blend_dir = base_path.clone();
        blend_dir.push(BLEND_DIR);

        // ensure path exists
        fs::create_dir_all(&render_dir).unwrap();
        fs::create_dir_all(&blend_dir).unwrap();

        Self {
            render_dir,
            blend_dir,
        }
    }
}

impl ServerSetting {
    pub fn get_config_dir() -> PathBuf {
        let path = dirs::config_dir().unwrap().join(SETTINGS_PATH);
        fs::create_dir_all(&path).expect("Unable to create directory!");
        path
    }

    fn get_config_path() -> PathBuf {
        let path = Self::get_config_dir();
        path.join(SETTINGS_FILE_NAME)
    }

    /// Save the configurations to the user's config directory.
    pub fn save(&self) -> IoResult<()> {
        let data = serde_json::to_string(&self).expect("Unable to parse ServerSettings into json!");
        let config_path = Self::get_config_path();
        fs::write(config_path, data)?;
        Ok(())
    }

    /// Load user configurations from the user's config directory
    pub fn load() -> ServerSetting {
        let config_path = Self::get_config_path();
        match fs::read_to_string(config_path) {
            Ok(data) => {
                let mut settings: ServerSetting =
                    serde_json::from_str(&data).expect("Unable to parse settings!");

                if !settings.render_dir.exists() || !settings.blend_dir.exists() {
                    settings = ServerSetting::default();
                    let _ = &settings.save();
                }

                settings
            }
            Err(_) => {
                let data = ServerSetting::default();
                let _ = &data.save();
                data
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_default_succeed() {
        let default = ServerSetting::default();

        let base_path = std::env::temp_dir();
        let mut render_dir = base_path.clone();
        render_dir.push(RENDER_DIR);

        let mut blend_dir = base_path.clone();
        blend_dir.push(BLEND_DIR);

        assert_eq!(default.blend_dir, blend_dir);
        assert_eq!(default.render_dir, render_dir);
        assert!(blend_dir.exists());
        assert!(render_dir.exists());
    }

    #[test]
    fn ensure_get_config_dir_succeed() {
        let config_path = ServerSetting::get_config_dir();
        assert!(config_path.exists());
    }

    #[test]
    fn ensure_get_config_path_succeed() {
        let path = ServerSetting::get_config_path();
        assert!(path.exists());
    }
}
