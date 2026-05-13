use crate::blender::Blender;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlenderConfig {
    /// List of installed blenders
    pub blenders: HashMap<Version, Blender>,

    /// Installation path. By default set to `$HOME/Downloads/Blender`
    pub install_path: PathBuf,
}

impl BlenderConfig {
    // pub fn load(file_path: impl AsRef<Path>) -> Result<BlenderConfig, Error> {
    //     let settings = Self::load();
    //     let mut config =
    //         settings.try_deserialize::<BlenderConfig>().unwrap();

    //     // let content = fs::read_to_string(&file_path)?;
    //     // let mut config = serde_json::from_str::<BlenderConfig>(&content)?;
    //     config.remove_invalid_blender();

    //     // TODO: Maybe we don't need this anymore if we're using config?
    //     // Look into watcher?
    //     config.inner = file_path.as_ref().to_path_buf();
    //     Ok(config)
    // }

    pub fn get_download_destination(&self, category_folder_name: &str) -> PathBuf {
        self.install_path.join(category_folder_name)
    }

    // Fetch best matching version of blender if provided, or latest version available if none was provided.
    pub fn get_latest_blender_available(&self, version: &Version) -> Option<&Blender> {
        self.get_blender(version)
            .or_else(|| self.get_blender_partial(version.major, version.minor))
    }

    /// Return matching exact blender version
    pub fn get_blender(&self, version: &Version) -> Option<&Blender> {
        self.blenders.values().find(|x| x.get_version().eq(version))
    }

    // return a immutable reference list of installed blender.
    // useful to display on website of some sort.
    pub fn get_blenders(&self) -> Vec<&Blender> {
        self.blenders
            .iter()
            .fold(Vec::new(), |mut map, (_, blender)| {
                map.push(blender);
                map
            })
    }

    /// Return a reference to matching partial version, but uses latest patch
    /// Major must match, Minor will match if greater than 0. Patch will always be the latest version possible.
    pub(crate) fn get_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.blenders
            .values()
            .fold(None, |latest: Option<&Blender>, item| {
                let current_version = item.get_version();

                if current_version.major.ne(&major) {
                    return latest;
                }

                // custom rule: If minor = 0 (default), use latest, otherwise compare all others.
                if minor > 0 && current_version.minor.ne(&minor) {
                    return latest;
                }

                if let Some(recent) = latest {
                    if recent.get_version().ge(current_version) {
                        return latest;
                    }
                }

                Some(item)
            })
    }

    /// Remove any invalid blender path entry from BlenderConfig
    pub fn remove_invalid_blender(&mut self) {
        self.blenders.retain(|_, v| v.get_executable().exists());
    }

    /// remove target blender
    pub fn remove_blender(&mut self, blender: &Blender) -> Option<Blender> {
        self.blenders.remove(blender.get_version())
    }

    /// Append blender entry to database
    /// This will create a new record if the key does not exist, or update record, returning old value.
    pub fn insert_blender(&mut self, blender: &Blender) -> Option<Blender> {
        // If Some returns, it means we override record. None means no previous record exist and a new entry is added.
        self.blenders
            .insert(blender.get_version().to_owned(), blender.clone())
    }
}

/* 
impl Default for BlenderConfig {
    fn default() -> Self {
        let install_path = dirs::download_dir()
        .expect("Must have place to download!")
        .join(SETTINGS_DIR);
    
    // ensure path location must exist to save and store to
    // - we've been given a place with permission access.
    if let Err(e) = fs::create_dir_all(&install_path) {
        eprintln!("Unable to create {e:?}");
    }
    
    Self {
        blenders: Default::default(),
        install_path,
    }
}
}
*/

impl Into<PathBuf> for BlenderConfig {
    fn into(self) -> PathBuf {
        self.install_path
    }
}
