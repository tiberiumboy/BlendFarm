use std::path::PathBuf;
use super::{args::{HardwareMode}, blender_scene::{BlenderScene, Sample}, device::Processor, engine::Engine, format::Format};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

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
    sample: Sample,
    pub(crate) engine: Engine,
    format: Format,
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
            sample: samples,
            engine,
            format,
            crop: false,
        }
    }
}