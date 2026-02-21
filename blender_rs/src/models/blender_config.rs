use std::path::PathBuf;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::blender::Blender;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderConfig {
    /// List of installed blenders
    blenders: Vec<Blender>,

    /// Install path. By default set to `$HOME/Downloads/Blender`
    pub install_path: PathBuf,

    /// Auto save on drop
    pub auto_save: bool,
}

impl BlenderConfig {
    pub fn new(blenders: Option<Vec<Blender>>, install_path: PathBuf, auto_save: bool) -> Self {
        match blenders {
            Some(vec) => Self {
                blenders: vec,
                install_path: install_path.into(),
                auto_save,
            },
            None => Self {
                blenders: Vec::new(),
                install_path: install_path.into(),
                auto_save,
            },
        }
    }

    /// Remove any invalid blender path entry from BlenderConfig
    pub fn remove_invalid_blender_path(&mut self) {
        self.blenders.retain(|x| x.get_executable().exists());
    }

    pub fn get_latest_blender_available(&self, version: Option<&Version>) -> Option<&Blender> {
        match version {
            Some(v) => self
                .blenders
                .iter()
                .filter(|b| b.get_version().ge(v))
                .collect::<Vec<&Blender>>()
                .first()
                .map(|v| &**v),
            None => self.blenders.first(),
        }
    }

    #[allow(dead_code)]
    pub fn get_auto_save(&self) -> &bool {
        &self.auto_save
    }

    // Don't think I need this function anymore?
    // pub fn get_blenders(&self) -> &Vec<Blender> {
    //     &self.blenders
    // }

    pub fn get_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.blenders.iter().find(|x| {
            let v = x.get_version();
            v.major.eq(&major) && v.minor.eq(&minor)
        })
    }

    pub fn get_blender(&self, version: &Version) -> Option<&Blender> {
        self.blenders.iter().find(|x| x.get_version().eq(version))
    }

    pub fn remove_blender(&mut self, blender: &Blender) {
        self.blenders.retain(|x| x.eq(blender));
    }

    pub fn append_blender(&mut self, blender: &Blender) {
        self.blenders.push(blender.clone());
        self.blenders.sort();
    }
}

impl Into<PathBuf> for BlenderConfig {
    fn into(self) -> PathBuf {
        self.install_path
    }
}
