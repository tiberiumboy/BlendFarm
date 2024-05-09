use semver::Version;
use serde::{Deserialize, Serialize};
use std::{env, fs::remove_file, io::Error, path::PathBuf, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectFile {
    pub file_name: String,
    pub src: PathBuf,
    #[serde(skip_serializing)]
    pub tmp: Option<PathBuf>,
    pub version: Version,
}

impl PartialEq for ProjectFile {
    fn eq(&self, other: &Self) -> bool {
        *self.src == *other.src
    }
}

#[allow(dead_code)]
impl ProjectFile {
    pub fn new(path: &PathBuf) -> Self {
        //let blend = Blend::from_path(path).expect("Unable to read blend file!");
        // todo find a way to detect what version this blend file was opened in
        let version = Version::new(4, 1, 0); //blend.version().unwrap();

        // in the path, I need to remove .blend from the path.
        Self {
            // TODO: Clean this up afterward!
            file_name: path
                .file_stem()
                // .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            src: path.to_owned(),
            tmp: None,
            version,
        }
    }

    pub fn parse(src: &str) -> Result<ProjectFile, Error> {
        let path = PathBuf::from(src);
        Ok(ProjectFile::new(&path))
    }

    pub(crate) fn move_to_temp(&mut self) {
        let mut dir = env::temp_dir();
        let file_name = self.src.file_name().unwrap();
        dir.push(file_name);
        let _ = std::fs::copy(&self.src, &dir);
        self.tmp = Some(dir);
    }

    pub(crate) fn clear_temp(&mut self) {
        if let Some(tmp) = &self.tmp {
            match remove_file(tmp) {
                Ok(_) => self.tmp = None,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }

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
