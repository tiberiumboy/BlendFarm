use std::{collections::HashMap, path::PathBuf};
use semver::Version;
use serde::{Deserialize, Serialize};
use crate::blender::Blender;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BlenderConfig {
    /// List of installed blenders
    blenders: HashMap<Version, Blender>,

    /// Install path. By default set to `$HOME/Downloads/Blender`
    pub install_path: PathBuf,
}

impl BlenderConfig {
    pub fn new(blenders: Option<Vec<Blender>>, install_path: PathBuf) -> Self {
        match blenders {
            Some(vec) => 
            Self {
                blenders: vec.iter().fold(HashMap::with_capacity(vec.capacity()), |mut accumulator, element| {
                let version = element.get_version().to_owned();
                accumulator.insert(version, element.to_owned());
                accumulator
            }),
                install_path: install_path.into(),
            },
            None => Self {
                blenders: HashMap::new(),
                install_path: install_path.into(),
            },
        }
    }

    pub fn get_download_destination(&self, category_folder_name: &str) -> PathBuf {
        self.install_path.join(category_folder_name)
    }

    // Fetch best matching version of blender if provided, or latest version available if none was provided.
    pub fn get_latest_blender_available(&self, version: Option<&Version>) -> Option<&Blender> {
        match version {
            Some(v) => {
                self.get_blender(v).or_else(|| self.get_blender_partial(v.major, v.minor))
            },
            None => self.blenders.iter().fold(None, |result, (version, blender)| {
                if let Some(current) = result {
                    if current.get_version().ge(version) {
                        return result;
                    }
                } 
                Some(blender)
            })
        }
    }

    /// Return matching exact blender version
    // TODO: Can we make this private?
    pub(crate) fn get_blender(&self, version: &Version) -> Option<&Blender> {
        self.blenders.values().find(|x| x.get_version().eq(version))
    }

    // return a immutable reference list of installed blender.
    // useful to display on website of some sort.
    pub(crate) fn get_blenders(&self) -> Vec<&Blender> {
        self.blenders.iter().fold(Vec::new(), |mut map, (_, blender)| {
            map.push(blender);
            map
        })
    }

    /// Return a reference to matching partial version, but uses latest patch
    /// Major must match, Minor will match if greater than 0. Patch will always be the latest version possible.
    // TODO: Can we make this private?
    pub(crate) fn get_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.blenders.values().fold(None, |latest: Option<&Blender>, item| {
            let current_version = item.get_version();
            if current_version.major.ne(&major) {
                return latest;
            }

            if match minor {
                0 => false,
                target => current_version.minor.ne(&target),
            } {
                return latest;
            }
            
            if let Some(recent) = latest {
                return match recent.get_version().ge(current_version) {
                    true => latest,
                    false => Some(item)
                }
            }

            Some(item)
        })
    }

    /// Update Blender installation location for installing blender package.
    pub fn update_install_path(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        // here we can do some things:
        // Future implementation: We can move all of the previous blender installation to the new path provided to us.
        // current implementation: Update pathbuf instead.
        self.install_path = path;
        Ok(())
    }

    /// Remove any invalid blender path entry from BlenderConfig
    pub fn remove_invalid_blender(&mut self) {
        self.blenders.retain(|_,v| v.get_executable().exists());
    }

    /// remove target blender
    pub fn remove_blender(&mut self, blender: &Blender) -> Option<Blender> {
        self.blenders.remove(blender.get_version())
    }

    /// Append blender entry to database
    /// This will create a new record if the key does not exist, or update record, returning old value.
    pub fn insert_blender(&mut self, blender: &Blender) -> Option<Blender> {
        // If Some returns, it means we override record. None means no previous record exist and a new entry is added.
        self.blenders.insert(blender.get_version().to_owned(), blender.clone())
    }
}

impl Into<PathBuf> for BlenderConfig {
    fn into(self) -> PathBuf {
        self.install_path
    }
}
