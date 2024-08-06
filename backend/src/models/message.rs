use std::net::SocketAddr;

use super::job::Job;
use semver::Version;
use serde::{Deserialize, Serialize};

// this command is provide by the User to the server.
// this interface acts as API - where we want to send command to the server node, and start taking actions.
#[derive(Debug)]
pub enum CmdMessage {
    AddPeer { name: String, socket: SocketAddr },
    SendJob(Job),
    AskForBlender { version: Version },
    // SetJobStatus(Uuid, NodeStatus), // target specific job to apply status to.
    Ping, // send a ping to the network
    Exit, // stop the thread process
}

// I could make this as a trait?
// that way I could have separate enum structs for different kind of message
#[derive(Serialize, Deserialize, Debug)]
pub enum NetMessage {
    // need to find a way to associate the completion of the job?
    // JobResult(RenderInfo), // return the result of the job]
    // From Clietn to Client
    CheckForBlender {
        os: String,
        arch: String,
        version: Version,
        caller: SocketAddr,
    },
    CanReceive(bool),

    // From Server to Client
    SendJob(Job),
    // RenderJob(RenderQueue),
    RequestJob,

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
