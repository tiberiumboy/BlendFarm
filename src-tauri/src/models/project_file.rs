/* 
use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    ops::Deref, path::{Path, PathBuf}, str::FromStr
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectFileError {
    // #[error("Invalid file type")]
    // InvalidFileType,
    #[error("Unexpected error - Programmer needs to specify exact error representation")]
    UnexpectedError, // should never happen.
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProjectFile<T: AsRef<Path>> {
    blender_version: Version,
    path: T,
}

impl ProjectFile<PathBuf> {
    pub fn new(src: PathBuf, version: Version) -> Result<Self, ProjectFileError> {
        match Blend::from_path(&src) {
            Ok(_data) => {
                Ok(Self {
                    blender_version: version,
                    path: src,
                })
            }
            Err(_) => Err(ProjectFileError::InvalidFileType),
        }
    }
}

impl AsRef<Version> for ProjectFile<PathBuf> {
    fn as_ref(&self) -> &Version {
        &self.blender_version
    }
}

impl FromStr for ProjectFile<PathBuf> {
    type Err = ProjectFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(s).map_err(|_| ProjectFileError::UnexpectedError)?)
    }
}

impl Deref for ProjectFile<PathBuf> {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

*/