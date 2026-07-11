use crate::blender::Frame;

use super::{
    args::HardwareMode,
    blender_scene::{BlenderScene, Sample},
    device::Processor,
    format::Format,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
// TODO: could rename this to something else?
pub struct BlenderConfiguration {
    // output various
    output: PathBuf,
    scene_info: BlenderScene,
    cores: usize,
    processor: Processor,
    hardware_mode: HardwareMode,
    sample: Sample,
    format: Format,
    start: Frame,
    end: Frame,
    // Py:- Value assign to use_crop_to_border, additionally, false set film_transparent true
    crop: bool,
}

impl BlenderConfiguration {
    pub fn new(
        output: PathBuf,
        scene_info: BlenderScene,
        processor: Processor,
        hardware_mode: HardwareMode,
        samples: Sample,
        format: Format,
        start: Frame,
        end: Frame,
    ) -> Self {
        let cores = match std::thread::available_parallelism() {
            Ok(f) => f.get(),
            Err(e) => {
                println!("{e:?}");
                1
            }
        };
        Self {
            output,
            scene_info,
            cores,
            processor,
            hardware_mode,
            sample: samples,
            format,
            crop: false,
            start,
            end,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::models::blender_scene::tests::mock_blender_scene;

    pub fn mock_blender_configuration() -> BlenderConfiguration {
        let blender_scene = mock_blender_scene();

        BlenderConfiguration {
            output: PathBuf::new(),
            scene_info: blender_scene,
            cores: 1,
            processor: Processor::NONE,
            hardware_mode: HardwareMode::CPU,
            sample: 100,
            format: Format::default(),
            start: 1,
            end: 2,
            crop: false,
        }
    }
}
