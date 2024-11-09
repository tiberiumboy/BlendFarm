use message_io::{
    network::Transport,
    node::{self, NodeTask},
};
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc;

use super::message::{CmdMessage, NetMessage};

pub enum NetworkError {
    NotConnected,
    Invalid,
    BadInput,
}

pub struct NetworkService {
    addr: SocketAddr,
    tx: mpsc::Sender<NetMessage>,
    rx: mpsc::Receiver<CmdMessage>,
    task: NodeTask,
}

impl NetworkService {
    pub async fn new(socket: SocketAddr) -> Result<Self, io::Error> {
        let (handler, listener) = node::split::<NetMessage>();

        let (task, /*mut*/ _receiver) = listener.enqueue();

        let (_resource_id, addr) = handler.network().listen(Transport::FramedTcp, socket)?; // would like to know what's the deal with unwrap function here...?
                                                                                            // two way communiucation to Netmessage. Transmitted
        let (tx, _rx) = mpsc::channel(); // Send message to network

        // two way communication from CmdMessage. Received.
        let (_tx_recv, rx_recv) = mpsc::channel(); // Receive command from network

        Ok(Self {
            tx,
            rx: rx_recv,
            addr,
            task,
        })
    }
}
