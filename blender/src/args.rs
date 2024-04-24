// use crate::device::Device;
// use crate::engine::Engine;
// use crate::format::Format;
use crate::mode::Mode;
use std::path::PathBuf;

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug)]
pub struct Args {
    pub background: bool, // optional
    pub file: PathBuf,    // required
    // pub engine: Option<Engine>, // optional
    // pub device: Option<Device>, // optional
    // pub format: Format,  // optional
    pub output: PathBuf, // optional
    pub mode: Mode,      // required
}

impl Args {
    pub fn new(file: PathBuf, output: PathBuf, mode: Mode) -> Self {
        Args {
            background: true,
            file,
            // engine: None,
            // device: None,
            output,
            mode,
        }
    }
}
