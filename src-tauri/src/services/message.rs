use std::{net::SocketAddr, path::PathBuf};
use serde::{Deserialize, Serialize};

// this command is provide by the User to the server.
// this interface acts as API - where we want to send command to the server node, and start taking actions.
pub enum ToNetwork {
    Connect( SocketAddr ),
    // how did the example do this? continuous file stream?
    SendFile(PathBuf),
    Ping{ host: bool }, // send a ping to the network
    Exit, // stop the thread process
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FromNetwork {
    SendFile(PathBuf), // might be expensive?
    RequestJob,
    // From multicast
    Ping {
        // server must provide address Some(SocketAddr), client send None
        server_addr: Option<SocketAddr>,
    },
}

// Net Message is used to send message to the client? What's the different from ToNode or FromNode?
#[derive(Debug,Serialize, Deserialize)]
pub enum NetMessage {
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


#[derive(Debug)]
pub enum NetResponse {
    Joined {
        #[allow(dead_code)]
        socket: SocketAddr,
    },
    Disconnected {
        #[allow(dead_code)]
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
    Ping,
    // JobSent(Job),
    // TODO: Find a good implementation to keep this enum, may have to change it to OnFileReceived?
    #[allow(dead_code)]
    FileTransfer(PathBuf),
}