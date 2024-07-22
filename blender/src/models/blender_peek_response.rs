use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlenderPeekResponse {
    #[serde(rename = "RenderWidth")]
    pub render_width: i32,
    #[serde(rename = "RenderHeight")]
    pub render_height: i32,
    #[serde(rename = "FrameStart")]
    pub frame_start: i32,
    #[serde(rename = "FrameEnd")]
    pub frame_end: i32,
    #[serde(rename = "FPS")]
    pub fps: u32,
    #[serde(rename = "Denoiser")]
    pub denoiser: String,
    #[serde(rename = "Samples")]
    pub samples: i32,
    #[serde(rename = "Cameras")]
    pub cameras: Vec<String>,
    #[serde(rename = "SelectedCamera")]
    pub selected_camera: String,
    #[serde(rename = "Scenes")]
    pub scenes: Vec<String>,
    #[serde(rename = "SelectedScene")]
    pub selected_scene: String,
}
