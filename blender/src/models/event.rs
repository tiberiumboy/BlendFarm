// use crate::blender::BlenderError;    // will use this for Error() enum variant.
use std::path::PathBuf;

#[derive(Debug)]
pub enum BlenderEvent {
    Log(String),
    Warning(String),
    Sample(String),
    Rendering{ current: f32, total: f32 },
    Completed { frame: i32, result: PathBuf },
    Unhandled(String),
    Exit,
    Error(String),
}