/*  DEV BLOG
[F] Run python script to extract information from blend file. ((plugins?Eevee/cycle?Cameras?Sample size?))
    - Consider this as a feature for future implementation - but now I need to ask tester for valuable information to extract from blender.

TODO: Is there a way to fetch the blender version the blend file was last opened in? I would like to automatically fill in the blanks from .blend file so the user doesn't have to
*/

use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectFileError {
    #[error("Invalid file type")]
    InvalidFileType,
    #[error("Unexpected error - Programmer needs to specify exact error representation")]
    UnexpectedError, // should never happen.
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProjectFile {
    file_name: String,
    blender_version: Version,
    path: PathBuf,
}

impl ProjectFile {
    pub fn new(src: PathBuf, version: Version) -> Result<Self, ProjectFileError> {
        match Blend::from_path(&src) {
            Ok(_data) => {
                let file_name = &src.file_name().unwrap();
                Ok(Self {
                    file_name: file_name.to_str().unwrap().to_string(),
                    blender_version: version,
                    path: src,
                })
            }
            Err(_) => Err(ProjectFileError::InvalidFileType),
        }
    }
}

impl AsRef<Path> for ProjectFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Version> for ProjectFile {
    fn as_ref(&self) -> &Version {
        &self.blender_version
    }
}

impl FromStr for ProjectFile {
    type Err = ProjectFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: ProjectFile = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
