use super::{file_info::FileInfo, render_info::RenderInfo, render_queue::RenderQueue};
use serde::{Deserialize, Serialize};

// I could make this as a trait?
// that way I could have separate enum structs for different kind of message
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    // From Client To Server
    RegisterNode {
        name: String,
    },
    UnregisterNode,
    // need to find a way to associate the completion of the job?
    JobResult(RenderInfo), // return the result of the job
    HaveBlender {
        os: String,
        version: String,
        arch: String,
    },

    // From Server to Client
    LoadJob(RenderQueue), // TODO figure out what kind of type I need to load here.
    // PrepareJob(Job),
    CancelJob,

    // From Client to Client
    // TODO: Future updates? - Let individual node module to share identical blender files over network instead of downloading from the server multiple of times.
    ContainBlenderResponse {
        have_blender: bool,
    },

    // From multicast
    // may need to split this?
    ServerPing,
    ClientPing {
        name: String,
    },
    FileRequest(FileInfo),
    // have a look into concurrent http file transfer if possible?
    Chunk(Vec<u8>), // how exactly can I make this server expects chunk of files?
    CanReceive(bool),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Signal {
    SendChunk,
}
