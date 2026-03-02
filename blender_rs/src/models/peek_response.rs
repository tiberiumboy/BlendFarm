use super::blender_scene::{BlenderScene, Camera, SceneName};
use crate::blender::Frame;
use semver::Version;
use serde::{Deserialize, Serialize};

// TODO: Find a way to get preference saved Processor from the file?
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PeekResponse {
    pub last_version: Version,
    pub current: BlenderScene,
    pub frame_start: Frame,
    pub frame_end: Frame,
    #[serde(rename = "FPS")]
    pub cameras: Vec<Camera>,
    pub scenes: Vec<SceneName>,
}

impl PeekResponse {
    pub fn new(
        last_version: Version,
        frame_start: Frame,
        frame_end: Frame,
        cameras: Vec<String>,
        scenes: Vec<String>,
        current: BlenderScene,
    ) -> Self {
        Self {
            last_version,
            frame_start,
            frame_end,
            cameras,
            scenes,
            current,
        }
    }
}
