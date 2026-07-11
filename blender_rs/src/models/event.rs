use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlenderEvent {
    Info(String),
    Rendering(RenderEvent),
    Warning(String),
    Exit,
    Error(String),
    Unhandled(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderEvent {
    Progress {
        frame: i32,
        current: f32,
        total: f32,
    },
    Complete {
        frame: i32,
        path: PathBuf,
    },
}
