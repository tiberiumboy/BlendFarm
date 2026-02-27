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

    // Seems like it's a read only mode?
    pub fn get_latest_blender_available(&self, version: Option<&Version>) -> Option<&Blender> {
        match version {
            // TODO: Finish this piece
            Some(v) => {
                self.blenders.values()
                    .filter(|b| b.get_version().ge(v))
                    .collect::<Vec<&Blender>>()
                    .first()
                    .map(|v| Some(v.to_owned()))?
            },
            None => self.blenders.iter().fold(None, |accumulator, item| {
                if let Some(b) = accumulator {
                    return match b.get_version().le(item.0) {
                        true => Some(&item.1),
                        false => accumulator
                    }
                } 
            
                Some(item.1)
            })


            // Some(v) => self
            //     .blenders
            //     .iter()
            //     .filter(|b| b.get_version().ge(v))
            //     .collect::<Vec<&Blender>>()
            //     .first()
            //     .map(|v| &**v),
            // None => self.blenders.first(),
        }
    }

    /// Return matching exact blender version
    pub fn get_blender(&self, version: &Version) -> Option<&Blender> {
        self.blenders.values().find(|x| x.get_version().eq(version))
    }

    /// Return a reference to matching partial version, but uses latest patch
    pub fn get_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.blenders.values().find(|x| {
            let v = x.get_version();
            v.major.eq(&major) && v.minor.eq(&minor)
        })
    }

    /// Remove any invalid blender path entry from BlenderConfig
    pub fn remove_invalid_blender_path(&mut self) {
        self.blenders.retain(|_,v| v.get_executable().exists());
    }

    /// remove target blender
    pub fn remove_blender(&mut self, blender: &Blender) -> bool {
        self.blenders.remove(blender.get_version()).is_some()
    }

    /// append blender to database
    pub fn append_blender(&mut self, blender: &Blender) -> Option<Blender> {
        // If Some returns, it means we override record. None means no previous record exist and a new entry is added.
        self.blenders.insert(blender.get_version().to_owned(), blender.clone())
    }
}

impl Into<PathBuf> for BlenderConfig {
    fn into(self) -> PathBuf {
        self.install_path
    }
}
