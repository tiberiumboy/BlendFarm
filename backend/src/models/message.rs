use std::net::SocketAddr;

use super::{job::Job, render_info::RenderInfo, render_queue::RenderQueue};
use serde::{Deserialize, Serialize};

// this command is provide by the User to the server.
// this interface acts as API - where we want to send command to the server node, and start taking actions.
pub enum CmdMessage {
    AddPeer { name: String, socket: SocketAddr },
    SendJob(Job),
    // SetJobStatus(Uuid, NodeStatus), // target specific job to apply status to.
    Ping, // send a ping to the network
    Exit, // stop the thread process
}

// I could make this as a trait?
// that way I could have separate enum structs for different kind of message
#[derive(Serialize, Deserialize, Debug)]
pub enum NetMessage {
    // From Client To Server
    RegisterNode {
        name: String,
    },
    UnregisterNode,

    // need to find a way to associate the completion of the job?
    JobResult(RenderInfo), // return the result of the job]

    // From Server to Client
    SendJob(Job),
    RenderJob(RenderQueue),

    // From multicast
    Ping {
        name: String,
        socket: SocketAddr,
        is_client: bool,
    },
}

impl NetMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(&data)
    }
}
