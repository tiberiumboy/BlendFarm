use crate::{
    blender::Blender,
    services::packages::{downloaded::Downloaded, package::PackageT, BlenderPath},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bundle {
    pub content: Downloaded,
    executable: PathBuf,
}

impl Bundle {
    pub(crate) fn new(content: Downloaded, executable: PathBuf) -> Self {
        Self {
            content,
            executable,
        }
    }
}

impl BlenderPath for Bundle {
    fn get_blender(&self) -> Option<Blender> {
        Blender::from_executable(&self.executable).ok()
    }
}

impl PackageT for Bundle {
    fn get_version(&self) -> &semver::Version {
        &self.content.origin.version
    }
}
