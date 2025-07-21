use blend::Blend;
use serde::{Deserialize, Serialize};
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectFileError {
    #[error("File type must be blend extension!")]
    InvalidFileType,
    #[error("Not a file!")]
    MustBeFile,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProjectFile {
    inner: PathBuf,
}

impl ProjectFile {
    pub fn new(src: PathBuf) -> Result<Self, ProjectFileError> {
        match Blend::from_path(&src) {
            Ok(_data) => Ok(Self { inner: src }),
            Err(_) => Err(ProjectFileError::InvalidFileType),
        }
    }
}

impl Into<PathBuf> for ProjectFile {
    fn into(self) -> PathBuf {
        self.inner
    }
}

impl FromStr for ProjectFile {
    type Err = ProjectFileError;

    // questionable?
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(s).map_err(|_| ProjectFileError::InvalidFileType)?)
    }
}

impl Deref for ProjectFile {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.inner
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_project_file_successfully() {
        let file = Path::new("./test.blend");
        let project_file = ProjectFile::new(file.to_path_buf());
        assert!(project_file.is_ok());
    }

    #[test]
    fn invalid_file_path_should_fail() {
        let file = Path::new("./dir");
        let project_file = ProjectFile::new(file.to_path_buf());
        assert!(project_file.is_err());
    }

    #[test]
    fn invalid_file_extension_should_fail() {
        let file = Path::new("./bad_extension.txt");
        let project_file = ProjectFile::new(file.to_path_buf());
        assert!(project_file.is_err());
    }
}
