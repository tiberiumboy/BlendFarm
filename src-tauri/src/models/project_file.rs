use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    id: String,
    title: String,
    src: PathBuf,
    tmp: PathBuf,
}

impl ProjectFile {
    pub fn create(src: PathBuf) -> Self {
        let mut dir = env::temp_dir();
        let file_name = src.file_name().unwrap();
        dir.push(&file_name);
        let _ = std::fs::copy(&src, &dir);

        Self {
            id: Uuid::new_v4().to_string(),
            title: file_name.to_str().unwrap().to_owned(),
            src: src.to_owned(),
            tmp: dir,
        }
    }
}
