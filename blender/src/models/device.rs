use serde::{Deserialize, Serialize};

/*
Developer blog-
The only reason why we need to add number that may or may not match blender's enum number list
is because we're passing in the arguments to the python file instead of Blender CLI.
Once I get this part of the code working, then I'll go back and refactor python code.
*/

#[derive(Debug, Clone, Deserialize, PartialEq)]
// TODO: Find a way to convert enum into String literal for json de/serialize
pub enum Processor {
    CPU,
    CUDA,
    HIP,
    OPENCL,
    ONEAPI,
    OPTIX,
}

impl Serialize for Processor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl Deserialize for Processor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(visitor)
    }
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
            _ => Processor::CPU,
        }
    }
}
