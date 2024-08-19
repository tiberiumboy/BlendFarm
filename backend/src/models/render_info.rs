use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct RenderInfo {
    pub frame: i32,
    pub path: PathBuf,
}

impl RenderInfo {
    pub fn new(frame: i32, path: &PathBuf) -> Self {
        Self {
            frame,
            path: path.clone(),
        }
    }
}
