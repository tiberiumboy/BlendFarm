use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use super::node::Node;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    // To Server
    RegisterNode { name: String, addr: SocketAddr },
    UnregisterNode { addr: SocketAddr },
    JobResult(String), // return the result of the job

    // From Server
    NodeList(Vec<(String, SocketAddr)>),
    LoadJob(), // TODO figure out what kind of type I need to load here.

    // From Client to Client
    // TODO: Future updates? - Let individual node module to share identical blender files over network instead of downloading from the server multiple of times.
    // HaveArchiveBlender {
    //     os: String,
    //     version: String,
    //     arch: String,
    // },
    // ExchangeBlender(String, usize),

    // From all
    FileRequest(String, usize),
    Chunk(Vec<u8>), // how exactly can I make this server expects chunk of files?
    CanReceive(bool),
}
