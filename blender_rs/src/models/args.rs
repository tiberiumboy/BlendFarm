/*
    Developer blog

    - Having done extensive research, Blender have two ways to interface to the program
        1. Through CLI
        2. Through Python API via "bpy" library

    Review online for possible solution to interface blender via CAPI, but was strongly suggested to use a python script instead
    this limits what I can do in term of functionality, but it'll be a good start.
    FEATURE - See if python allows pointers/buffer access to obtain job render progress - Allows node to send host progress result. Possibly viewport network rendering?

    Do note that blender is open source - it's not impossible to create FFI that interfaces blender directly, but rather, there's no support to perform this kind of action (yet).
*/
// May Subject to change.
use super::device::Processor;
use crate::{
    blend_file::BlendFile,
    blender::Frame,
    models::{config::BlenderConfiguration, format::Format, peek_response::PeekResponse},
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Blender 4.2 introduce a new enum called BLENDER_EEVEE_NEXT, which is currently handle in python file atm.
// const EEVEE_SWITCH: Version = Version::new(4, 2, 0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HardwareMode {
    CPU,
    GPU,
    BOTH,
}

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
/// Field must be public to offer context to render the scene. Let user mutate however they see fits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Args {
    pub file: BlendFile, // required
    pub output: PathBuf, // optional
    pub processor: Processor,
    pub mode: HardwareMode, // optional
    pub format: Format,     // optional - default to Png
    pub start: Frame,
    pub end: Frame,
}

impl Args {
    pub fn new(file: BlendFile, output: PathBuf, start: Frame, end: Frame) -> Self {
        Args {
            file: file,
            output: output,
            processor: Processor::NONE,
            mode: HardwareMode::CPU,
            format: Format::default(),
            start,
            end,
        }
    }

    /// Args are user provided value - this should not correlate to the machine's hardware (CUDA/OPTIX/GPU usage)
    pub fn parse_from(&self, version: Option<&Version>) -> BlenderConfiguration {
        let info: PeekResponse = self.file.peek_response(version);
        BlenderConfiguration::new(
            self.output.clone(),
            info.current.clone(),
            self.processor.clone(),
            self.mode.clone(),
            info.current.render_setting.sample,
            info.current.render_setting.format,
            self.start,
            self.end,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blend_file::tests::mock_blend_file;

    #[test]
    fn success_args_builder() {
        let mock_blend_file = mock_blend_file();
        // Must have blendfile exist and verify integrity usage.
        let output = PathBuf::new(); // TOOD: Find a way to reference ./blender_rs/examples/assets/test.blend without reference/abs path chaoticness
        let args = Args::new(mock_blend_file.clone(), output.clone(), 1, 2);
        assert_eq!(args.file, mock_blend_file);
        assert_eq!(args.output, output);
        assert_eq!(args.start, 1);
        assert_eq!(args.end, 2);
    }
}
