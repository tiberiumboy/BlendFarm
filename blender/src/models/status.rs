use std::path::PathBuf;

// TODO Find good use of this?
#[derive(Debug)]
pub enum Status {
    Idle,
    Running { status: String },
    Error { message: String },
    Completed { result: PathBuf }, // should this be a pathbuf instead? or the actual image data?
}
