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
use crate::{
    blend_file::BlendFile, blender::Frame, models::{config::BlenderConfiguration, engine::Engine, format::Format, peek_response::PeekResponse}
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::device::Processor;

// Blender 4.2 introduce a new enum called BLENDER_EEVEE_NEXT, which is currently handle in python file atm.
const EEVEE_SWITCH: Version = Version::new(4, 2, 0);

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
    pub engine: Engine,  // optional
    pub processor: Processor,
    pub mode: HardwareMode, // optional
    pub format: Format,     // optional - default to Png
    pub start: Frame,
    pub end: Frame,
}

impl Args {
    pub fn new(file: BlendFile, output: PathBuf, engine: Engine, start: Frame, end: Frame) -> Self {
        Args {
            file: file,
            output: output,
            processor: Processor::NONE,
            mode: HardwareMode::CPU,
            engine,
            format: Format::default(),
            start,
            end
        }
    }

    /// Args are user provided value - this should not correlate to the machine's hardware (CUDA/OPTIX/GPU usage)
    pub fn parse_from(&self, version: &Version) -> BlenderConfiguration {
        let info: PeekResponse = self.file.peek_response(Some(version));
        BlenderConfiguration::new(
            self.output.clone(),
            info.current.clone(),
            self.processor.clone(),
            self.mode.clone(),
            info.current.render_setting.sample,
            match info.current.render_setting.engine {
                Engine::BLENDER_EEVEE | Engine::BLENDER_EEVEE_NEXT => {
                    if version.ge(&EEVEE_SWITCH) {
                        Engine::BLENDER_EEVEE_NEXT
                    } else {
                        Engine::BLENDER_EEVEE
                    }
                }
                _ => info.current.render_setting.engine
            },
            info.current.render_setting.format,
        )
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Need to write a unit test to ensure the correct engine is used per blender version.
    #[test]
    fn blender_test_eevee_engine_enum_switch() {
        // let file = 
        // TODO: How can I mock up a blendfile for unit test?
        // reference it from blendfile?
        let path_to_blend_file = PathBuf::from("./examples/assets/test.blend");
        // TODO: Create a mock blendfile for unit testing purposes.
        let file = BlendFile::new(&path_to_blend_file).expect("Must have a valid blend file!");
        let output = PathBuf::new();
        let engine = Engine::BLENDER_EEVEE_NEXT;
        let args = Args::new(file, output, engine, 1,1 );
        let parsed = args.parse_from(&Version::new(4,1,0));
        assert_ne!(parsed.engine, engine);
        let parsed = args.parse_from(&EEVEE_SWITCH);
        assert_eq!(parsed.engine, engine);
    }
}
