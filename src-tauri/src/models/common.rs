use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// not sure if I still need this or keep it separated?
#[derive(Serialize, Deserialize)]
pub enum SenderMsg {
    FileRequest(String, usize),
    Chunk(Vec<u8>),
    Render(PathBuf, i32),
}

#[derive(Serialize, Deserialize)]
pub enum ReceiverMsg {
    CanReceive(bool),
}
