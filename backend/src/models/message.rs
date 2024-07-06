use crate::models::job::Job;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    // From Client To Server
    RegisterNode {
        name: String,
        addr: SocketAddr,
    },
    UnregisterNode {
        addr: SocketAddr,
    },
    JobResult(String), // return the result of the job
    HaveBlender {
        os: String,
        version: String,
        arch: String,
    },

    // From Server to Client
    NodeList(HashMap<SocketAddr, String>),
    LoadJob(Job), // TODO figure out what kind of type I need to load here.
    PrepareJob(Job),

    // From Client to Client
    // TODO: Future updates? - Let individual node module to share identical blender files over network instead of downloading from the server multiple of times.
    ContainBlenderResponse {
        have_blender: bool,
    },

    // From multicast
    ServerPing {
        port: u16,
    },
    FileRequest(PathBuf, usize),
    // have a look into concurrent http file transfer if possible?
    Chunk(Vec<u8>), // how exactly can I make this server expects chunk of files?
    CanReceive(bool),
}
