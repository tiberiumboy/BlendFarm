use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlenderPeekResponse {
    pub last_version: Version,
    pub render_width: i32,
    pub render_height: i32,
    pub frame_start: i32,
    pub frame_end: i32,
    #[serde(rename = "FPS")]
    pub fps: u16,
    pub samples: i32,
    pub cameras: Vec<String>,
    pub selected_camera: String,
    pub scenes: Vec<String>,
    pub selected_scene: String,
    pub engine: String,
    pub output: String,
}
