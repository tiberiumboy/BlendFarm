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
    pub denoiser: String,
    pub samples: i32,
    pub cameras: Vec<String>,
    pub selected_camera: String,
    pub scenes: Vec<String>,
    pub selected_scene: String,
    // TODO: Found a way to save the current engine used in Blender. make this option available as soon as we fix peek.py to use blend lib instead.
    // pub engine: String,
}
