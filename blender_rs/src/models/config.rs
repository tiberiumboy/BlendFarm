use std::path::PathBuf;
use super::{args::{Args, HardwareMode}, blender_scene::{BlenderScene, Sample}, device::Processor, engine::Engine, format::Format, peek_response::PeekResponse};
use semver::Version;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

// Blender 4.2 introduce a new enum called BLENDER_EEVEE_NEXT, which is currently handle in python file atm.
const EEVEE_SWITCH: Version = Version::new(4, 2, 0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlenderConfiguration {
    #[serde(rename = "TaskID")]
    id: Uuid,
    // output various
    output: PathBuf,
    scene_info: BlenderScene,
    cores: usize,
    processor: Processor,
    hardware_mode: HardwareMode,
    // TODO: May be phased out?
    tile_width: i32,
    tile_height: i32,
    sample: Sample,
    engine: Engine,
    format: Format,
    // Py:- Value assign to use_crop_to_border, additionally, false set film_transparent true
    crop: bool,
}

impl BlenderConfiguration {
    fn new(
        output: PathBuf,
        scene_info: BlenderScene,
        processor: Processor,
        hardware_mode: HardwareMode,
        tile_width: i32,
        tile_height: i32,
        samples: Sample,
        engine: Engine,
        format: Format,
    ) -> Self {
        let cores = match std::thread::available_parallelism() {
            Ok(f) => f.get(),
            Err(e) => {
                println!("{e:?}");
                1
            }
        };
        Self {
            id: Uuid::new_v4(),
            output,
            scene_info,
            cores,
            processor,
            hardware_mode,
            tile_width,
            tile_height,
            sample: samples,
            engine,
            format,
            crop: false,
        }
    }

    /// Args are user provided value - this should not correlate to the machine's hardware (CUDA/OPTIX/GPU usage)
    pub fn parse_from(args: &Args, version: &Version) -> Self {
        let info: PeekResponse = args.file.peek_response(Some(version));
        BlenderConfiguration::new(
            args.output.clone(),
            info.current.clone(),
            args.processor.clone(),
            args.mode.clone(),
            -1,
            -1,
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
