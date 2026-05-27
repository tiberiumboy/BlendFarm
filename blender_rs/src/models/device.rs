use serde::{Deserialize, Serialize};

/*
Developer blog-
The only reason why we need to add number that may or may not match blender's enum number list
is because we're passing in the arguments to the python file instead of Blender CLI.
Once I get this part of the code working, then I'll go back and refactor python code.
*/

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// TODO: Find a way to convert enum into String literal for json de/serialize
pub enum Processor {
    NONE,
    CUDA,
    OPTIX,
    HIP,
    ONEAPI,
    // is there METAL?
}
