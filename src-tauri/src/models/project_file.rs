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
    
    // pathbuf must be validate, therefore method must be private
    fn new(src: PathBuf) -> Self {
        Self {
            inner: src
        }
    }

    /// Validate path integrity
    pub fn from<P>(src: P) -> Result<Self, ProjectFileError> 
    where P: AsRef<Path> 
    {
        let path = src.as_ref();
        
        // Blend expects a file. Stop here if argument is a directory. Do not continue.
        if path.is_dir() {
            return Err(ProjectFileError::InvalidFileType)
        }

        if !path.exists() {
            return Err(ProjectFileError::InvalidFileType)
        }
        
        // expects a file existing, do not pass in directory or this program will crash.
        if let Err(e) = Blend::from_path(path) {
            eprintln!("{e:?}");
            return Err(ProjectFileError::InvalidFileType)
        };

        let buf = path.to_path_buf();
        Ok(Self::new(buf))
    }
}

/* #region custom implementation */

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

/* #endregion */

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::constant::test::EXAMPLE_FILE;
    use std::path::Path;

    #[test]
    fn create_project_file_successfully() {
        let file = Path::new(EXAMPLE_FILE);
        let project_file = ProjectFile::from(file.to_path_buf());
        assert!(project_file.is_ok());
    }

    #[test]
    fn invalid_file_path_should_fail() {
        let file = Path::new("./dir");
        let project_file = ProjectFile::from(file.to_path_buf());
        assert!(project_file.is_err());
    }

    #[test]
    fn invalid_file_extension_should_fail() {
        // with invalid extension (e.g. .txt)
        {
            let file = Path::new("./bad_extension.txt");
            let project_file = ProjectFile::from(file.to_path_buf());
            assert!(project_file.is_err());
        }

        // with no extension (e.g. dir)
        {
            let dir = Path::new("./");
            let project_file = ProjectFile::from(dir);
            assert!(project_file.is_err());
        }
    }
}
