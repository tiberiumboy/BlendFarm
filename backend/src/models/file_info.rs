use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: usize,
}

impl FileInfo {
    #[allow(dead_code)]
    pub fn new(path: PathBuf) -> Self {
        let size = fs::metadata(&path).unwrap().len() as usize;
        let name = path
            .file_name()
            .expect("Missing file name! This should not happen!")
            .to_os_string()
            .into_string()
            .unwrap();

        Self { name, path, size }
    }
}