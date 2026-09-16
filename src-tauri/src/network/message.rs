// use futures::channel::oneshot::{self};
use crate::services::server::ServerEvent;
use libp2p::Multiaddr;

#[derive(Debug)]
pub enum NetworkError {
    UnableToListen(String),
    NotConnected,
    Invalid,
    BadInput,
    SendError(String),
    NoPeerProviderFound,
    UnableToSave(String),
    Timeout,
}

pub type KeywordSearch = String;

// Send commands to network.
// TODO: Make two different kind of message, one use for broadcast and the other for direct communication.
#[derive(Debug)]
pub enum Command {
    // Subscribe {
    //     topic: String,
    // },
    // StartListening {
    //     addr: Multiaddr,
    //     sender: oneshot::Sender<FileResult<()>>,
    // },
    // StopListening,
    // Message this peer with server events. (Consider looking into receiving NetworkRequest enum?)
    // These are signal to use to send out message and forget.
    // May receive a response back, using the direct message above.
    Message(Option<Multiaddr>, ServerEvent),
}

// // Received network events.
// #[derive(Debug)]
// pub enum Event {
//     // When the node becomes available on the network.
//     Discovered(PeerId, Multiaddr),
//     // this is used for file transfer protocol
//     InboundRequest {
//         request: String,
//         channel: ResponseChannel<FileResponse>,
//     },
//     // may not actually need this?
//     ServerStatus(ServerEvent),
//     // may not actually need this?
//     JobUpdate(JobEvent),
//     // Used to exchange file data
//     ReceivedFileData(OutboundRequestId, Vec<u8>),
// }
