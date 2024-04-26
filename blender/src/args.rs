// use crate::device::Device;
// use crate::engine::Engine;
// use crate::format::Format;
use crate::mode::Mode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug, Serialize, Deserialize)]
pub struct Args {
    pub file: PathBuf, // required
    // pub engine: Option<Engine>, // optional
    // pub device: Option<Device>, // optional
    // pub format: Format,  // optional
    pub output: PathBuf, // optional
    pub mode: Mode,      // required
}

impl Args {
    pub fn new(file: PathBuf, output: PathBuf, mode: Mode) -> Self {
        Args {
            file,
            // engine: None,
            // device: None,
            output,
            mode,
        }
    }
}
