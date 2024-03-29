use crate::services::sender::run as send;
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf, str::FromStr};
use uuid::Uuid;

use super::render_node::RenderNode;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    id: String,
    src: PathBuf,
    #[serde(skip_serializing)]
    tmp: Option<PathBuf>,
}

impl ProjectFile {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            src: path.to_owned(),
            tmp: None,
        }
    }

    pub fn parse(src: &str) -> Result<ProjectFile, std::io::Error> {
        let path = PathBuf::from(src);
        Ok(ProjectFile::new(&path))
    }

    pub(crate) fn move_to_temp(&mut self) {
        let mut dir = env::temp_dir();
        let file_name = self.src.file_name().unwrap();
        dir.push(&file_name);
        let _ = std::fs::copy(&self.src, &dir);
        self.tmp = Some(dir);
    }

    pub(crate) fn clear_temp(&mut self) {
        if let Some(tmp) = &self.tmp {
            let _ = std::fs::remove_file(tmp);
        }
        self.tmp = None;
    }

    pub(crate) fn upload(&self, render_nodes: Vec<RenderNode>) {
        for node in render_nodes {
            // send file to node
            send(&self.src, &node);
        }
    }
}

impl FromStr for ProjectFile {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: ProjectFile = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
