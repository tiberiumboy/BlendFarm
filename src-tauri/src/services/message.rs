use std::{collections::HashSet, net::SocketAddr, path::PathBuf};
use message_io::network::Endpoint;
use semver::Version;
use serde::{Deserialize, Serialize};
use crate::models::{job::Job, render_node::RenderNode};

// I disagree with this...
pub enum Destination {
    Target(Endpoint),
    All,
}

// this command is provide by the User to the server.
// this interface acts as API - where we want to send command to the server node, and start taking actions.
pub enum ToNetwork {
    Connect( SocketAddr ),
    // how did the example do this? continuous file stream?
    SendFile(String, Vec<u8>),
    GetPeers, // return the list of peers connected to the server
    Ping, // send a ping to the network
    Exit, // stop the thread process
}

// I could make this as a trait?
// that way I could have separate enum structs for different kind of message
#[derive(Serialize, Deserialize, Debug)]
pub enum FromNetwork {
    // need to find a way to associate the completion of the job?
    // JobResult(RenderInfo), // return the result of the job]
    // From Clietn to Client
    // From Server to Client
    // SendJob(Job),
    SendFile(String, Vec<u8>), // might be expensive?
    RequestJob,
    // From multicast
    Ping {
        // server must provide address Some(SocketAddr), client send None
        server_addr: Option<RenderNode>,
    },
}

impl FromNetwork {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(&data)
    }
}

// this message is used from client side to communicate back to the service. We're receiving command to invoke actions
// why can't we use rpc? How can we 
// todo figure out what I can do with this one?
#[derive(Debug)]
pub enum CmdCommand {
    FileReceived {
        path: PathBuf
    },
    Download( Version ),
    Upload (PathBuf),
    Render { job: Job } // TODO: I know we have a special struct for this - find that and use that instead of Job.
}

// Net Message is used to send message to the client? What's the different from ToNode or FromNode?
#[derive(Debug,Serialize, Deserialize)]
pub enum NetMessage {
    AddPeer { socket: SocketAddr },
    SendFile(String, Vec<u8>),
    RequestJob,
    Ping(Option<SocketAddr>),
}

impl NetMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(&data)
    }
}

// pub enum Signal {
//     Stream(Option<(Vec<u8>, usize, usize)>)
// }

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
    Ping,
    // JobSent(Job),
    // TODO: Find a good implementation to keep this enum, may have to change it to OnFileReceived?
    #[allow(dead_code)]
    FileTransfer(PathBuf),
}