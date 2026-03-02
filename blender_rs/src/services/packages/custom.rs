use std::path::{Path, PathBuf};
use semver::Version;
use serde::{Deserialize, Serialize};
use crate::{blender::{Blender, BlenderError}, services::packages::{BlenderPath, package::PackageT}};

/// Design to let user upload path to blender executables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Custom {
    version: Version,
    executable: PathBuf
}

impl Custom {
    pub fn new(path: impl AsRef<Path> ) -> Result<Self, BlenderError> {
        let blender = Blender::from_executable(path)?;
        Ok(Self {
            version: blender.get_version().to_owned(),
            executable: blender.get_executable().to_owned()
        })
    }
}

impl BlenderPath for Custom {
    fn get_blender(&self) -> Option<Blender> {
        Blender::from_executable(&self.executable).ok()
    }
}

impl PackageT for Custom {
    fn get_version(&self) -> &semver::Version {
        &self.version
    }
}
