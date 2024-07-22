use serde::{Deserialize, Serialize};

/*
Developer blog-
The only reason why we need to add number that may or may not match blender's enum number list
is because we're passing in the arguments to the python file instead of Blender CLI.
Once I get this part of the code working, then I'll go back and refactor python to make this less ugly and hackable.
*/

// TODO: Once python code is working with this rust code - refactor python to reduce this garbage mess below:
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[allow(dead_code, non_camel_case_types)]
pub enum Device {
    #[default]
    CPU = 0,
    CUDA = 1,
    OPENCL = 2,
    CUDA_GPUONLY = 3,
    OPENCL_GPUONLY = 4,
    HIP = 5,
    HIP_GPUONLY = 6,
    METAL = 7,
    METAL_GPUONLY = 8,
    ONEAPI = 9,
    ONEAPI_GPUONLY = 10,
    OPTIX = 11,
    OPTIX_GPUONLY = 12,
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
            _ => todo!("to be implemented after getting this to work with python!"),
        }
    }
}
