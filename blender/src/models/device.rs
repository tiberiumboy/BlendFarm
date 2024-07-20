use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub enum Device {
    #[default]
    CPU,
    CUDA,
    OPTIX,
    HIP,
    ONEAPI,
    METAL,
}

// Append +CPU to a GPU device to render on both CPU and GPU.
impl ToString for Device {
    fn to_string(&self) -> String {
        match self {
            Device::CPU => "CPU".to_owned(),
            Device::CUDA => "CUDA".to_owned(),
            Device::OPTIX => "OPTIX".to_owned(),
            Device::HIP => "HIP".to_owned(),
            Device::ONEAPI => "ONEAPI".to_owned(),
            Device::METAL => "METAL".to_owned(),
        }
    }
}
