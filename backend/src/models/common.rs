use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum SenderMsg {
    FileRequest(String, usize),
    Chunk(Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub enum ReceiverMsg {
    CanReceive(bool),
}
