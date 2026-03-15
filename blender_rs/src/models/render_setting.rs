use super::{blender_scene::Sample, /* engine::Engine, */ format::Format, window::Window};
use crate::blender::Frame;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type FrameRate = u16; // u32 convert into string for xml-rpc. BEWARE!

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSetting {
    /// output of where our stored image will save to
    output: PathBuf,
    /// Render frame Width
    pub width: Frame, // Not to be confused with animation frame
    /// Render frame height
    pub height: Frame, // Not to be confused with animation frame
    /// Samples capture from the scene
    pub sample: Sample,
    /// Frame per second
    #[serde(rename = "FPS")]
    pub fps: FrameRate,
    /// What render engine to use (Optix/CUDA)
    // pub engine: Engine,
    /// Image format
    pub format: Format,
    /// Borders
    pub border: Window,
}

impl RenderSetting {
    pub fn new(
        output: PathBuf,
        width: Frame,
        height: Frame,
        sample: Sample,
        fps: FrameRate,
        /* engine: Engine,*/ format: Format,
        border: Window,
    ) -> Self {
        Self {
            output,
            width,
            height,
            sample,
            fps,
            // engine,
            format,
            border,
        }
    }

    pub fn set_output(mut self, output: PathBuf) -> Self {
        self.output = output;
        self
    }

    pub fn get_output(&self) -> &PathBuf {
        &self.output
    }
}
