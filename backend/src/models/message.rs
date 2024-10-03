use std::{collections::HashSet, net::SocketAddr, path::PathBuf};

use super::job::Job;
use message_io::network::Endpoint;
use semver::Version;
use serde::{Deserialize, Serialize};

pub enum Destination {
    Target(Endpoint),
    All,
}

// this command is provide by the User to the server.
// this interface acts as API - where we want to send command to the server node, and start taking actions.
pub enum CmdMessage {
    AddPeer {
        name: String,
        socket: SocketAddr,
    },
    SendJob(Job),
    SendFile(PathBuf, Destination),
    #[allow(dead_code)]
    AskForBlender {
        version: Version,
    },
    // return the list of peers connected to the server
    GetPeers,
    // SetJobStatus(Uuid, NodeStatus), // target specific job to apply status to.
    Ping, // send a ping to the network
    Exit, // stop the thread process
}

#[derive(Debug)]
pub enum NetResponse {
    Joined {
        socket: SocketAddr,
    },
    Disconnected {
        socket: SocketAddr,
    },
    #[allow(dead_code)]
    Info {
        socket: SocketAddr,
        name: String,
    }, // TODO: provide more context and list here once we get this working
    #[allow(dead_code)]
    Status {
        socket: SocketAddr,
        status: String,
    },
    PeerList {
        addrs: HashSet<Endpoint>,
    },
    JobSent(Job),
    // TODO: Find a good implementation to keep this enum, may have to change it to OnFileReceived?
    #[allow(dead_code)]
    ImageComplete(PathBuf),
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
    SendFile(String, Vec<u8>), // might be expensive?
    RequestJob,

    // From multicast
    Ping {
        // server would provide the address
        server_addr: Option<SocketAddr>,
    },
}

impl NetMessage {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(&data)
    }
}
