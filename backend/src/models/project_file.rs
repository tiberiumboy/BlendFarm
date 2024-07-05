use serde::{Deserialize, Serialize};
use std::{env, fs::remove_file, path::PathBuf, str::FromStr};

// TODO: this may ultimately get removed? We just need the pathbuf to the blender file specifically..
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectFile {
    pub src: PathBuf,
    #[serde(skip_serializing)]
    pub tmp: Option<PathBuf>,
}

impl PartialEq for ProjectFile {
    fn eq(&self, other: &Self) -> bool {
        *self.src == *other.src
    }
}

impl ProjectFile {
    pub fn new(src: PathBuf) -> Self {
        // in the path, I need to remove .blend from the path.
        Self { src, tmp: None }
    }

    // pub fn parse(src: &str) -> Result<ProjectFile, Error> {
    //     let path = PathBuf::from(src);
    //     Ok(ProjectFile::new(path))
    // }

    pub(crate) fn move_to_temp(&mut self) {
        // TODO: Do not use temp_dir() - MacOS clear the temp directory after a restart! BAD!
        let mut dir = env::temp_dir();
        let file_name = self.src.file_name().unwrap();
        dir.push(file_name);
        if let Ok(_) = std::fs::copy(&self.src, &dir) {
            self.tmp = Some(dir);
        }
    }

    /// Retrieve the file name from source path
    // pub fn get_file_name(&self) -> String {
    //     self.src.file_stem().unwrap().to_str().unwrap().to_owned()
    // }

    pub(crate) fn clear_temp(&mut self) {
        if let Some(tmp) = &self.tmp {
            match remove_file(tmp) {
                Ok(_) => self.tmp = None,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }

    // I find this useful to return the path of the recent file location.
    #[allow(dead_code)]
    pub(crate) fn file_path(&self) -> &PathBuf {
        self.tmp.as_ref().unwrap_or(&self.src)
    }
}

impl FromStr for ProjectFile {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: ProjectFile = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
