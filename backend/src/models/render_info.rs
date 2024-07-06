use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct RenderInfo {
    pub frame: i32,
    pub path: PathBuf,
}

impl RenderInfo {
    #[allow(dead_code)]
    pub fn new(frame: i32, path: &PathBuf) -> Self {
        Self {
            frame,
            path: path.clone(),
        }
    }
}
