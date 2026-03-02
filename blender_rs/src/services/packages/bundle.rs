use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::{blender::Blender, services::packages::{BlenderPath, downloaded::Downloaded, package::PackageT}};


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bundle {
    content: Downloaded,
    executable: PathBuf
}

impl Bundle {
    pub(crate) fn new(content: Downloaded, executable: PathBuf ) -> Self {
        Self {
            content,
            executable
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