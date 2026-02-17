use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlenderEvent {
    Log(String),
    Warning(String),
    Rendering { current: f32, total: f32 },
    Completed { frame: i32, result: PathBuf },
    Unhandled(String),
    Exit,
    Error(String),
}

// impl BlenderEvent {

// }
