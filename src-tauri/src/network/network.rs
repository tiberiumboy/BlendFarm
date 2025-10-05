use crate::models::computer_spec::ComputerSpec;
use crate::network::PeerIdString;
use blender::models::event::BlenderEvent;
use serde::{Deserialize, Serialize};

/*
Network Service - Receive, handle, and process network request.
*/
// why does the transfer have number at the trail end? look more into this?
const TRANSFER: &str = "/file-transfer/1";


// what is StatusEvent responsibility?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusEvent {
    Offline,
    Online,
    Busy,
    Error(String),
    Signal(String),
}

// Must be serializable to send data across network
// issue with this is that this cannot be convert into Encode,Decode by bincode. Instead we'll have to
#[derive(Debug, Serialize, Deserialize)]
pub enum NodeEvent {
    Hello(PeerIdString, ComputerSpec),
    Disconnected {
        peer_id: PeerIdString,
        reason: Option<String>,
    },
    BlenderStatus(BlenderEvent),
}