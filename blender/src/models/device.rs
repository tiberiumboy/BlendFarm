use serde::{Deserialize, Serialize};

/*
Developer blog-
The only reason why we need to add number that may or may not match blender's enum number list
is because we're passing in the arguments to the python file instead of Blender CLI.
Once I get this part of the code working, then I'll go back and refactor python to make this less ugly and hackable.
*/

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// TODO: Find a way to convert enum into String literal for json de/serialize
pub enum Processor {
    CPU,
    CUDA,
    HIP,
    OPENCL,
    ONEAPI,
    OPTIX
}

// TODO: Find a way to serialize/deserialize into correct values
impl Processor {
    fn as_str(&self) -> &'static str {
        match self {
            Processor::CPU => "CPU",
            Processor::CUDA => "CUDA",
            Processor::HIP => "HIP",
            Processor::OPENCL => "OPENCL",
            Processor::ONEAPI => "ONEAPI",
            Processor::OPTIX => "OPTIX",
        }
    }

    fn from_str(str: &str) -> Self {
        match str {
            "CUDA" => Processor::CUDA,
            "HIP" => Processor::HIP,
            "OPENCL" => Processor::OPENCL,
            "ONEAPI" => Processor::ONEAPI,
            "OPTIX" => Processor::OPTIX,
            _ => Processor::CPU
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RenderKind {
    processor: Processor,
    use_cpu: bool,
    use_gpu: bool,
    device: String
}

impl RenderKind {
    pub fn new(processor: Processor, use_gpu: bool ) -> Self {
        // The only time I ever see this use is for the python function "useDevices(kind, gpu, cpu)"
        let use_cpu = processor == Processor::CPU;
        let device = match use_cpu {
            true => "CPU", 
            _ => "GPU",
        }.to_owned();

        Self {
            processor,
            use_cpu,
            use_gpu,
            device  
        }
    }
}