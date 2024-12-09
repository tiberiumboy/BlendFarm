use crate::blender::BlenderError;
use std::path::PathBuf;

// TODO Find good use of this?
#[derive(Debug)]
pub enum Status {
    Idle,
    Running { status: String },
    Log { status: String },
    Warning { message: String },
    Error(BlenderError),
    Completed { frame: i32, result: PathBuf }, // should this be a pathbuf instead? or the actual image data?
}
