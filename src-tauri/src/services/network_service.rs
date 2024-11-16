/*

the idea behind this is to have a persistence model to contain network services. 
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.

 */

use message_io::{
    network::Transport,
    node::{self, NodeTask},
};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::{io, marker::PhantomData};

use super::message::{CmdMessage, NetMessage};

pub enum NetworkError {
    NotConnected,
    Invalid,
    BadInput,
}

struct Online;
struct Offline;


// can we use phantom state here?
pub struct NetworkService<State = Offline> {
    addr: SocketAddr,
    tx: mpsc::Sender<NetMessage>,
    rx: mpsc::Receiver<CmdMessage>,
    task: NodeTask,
    state: PhantomData<State>,
}

impl NetworkService<Offline> {
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
            state: PhantomData<Offline>::new(),
        })
    }
}

impl NetworkService<Online> {
    
}

impl NetworkService {
    pub fn new(socket: SocketAddr) -> Self{
        let ( tx, rx ) = mpsc::channel<NetMessage>();

        let task = std::thread::spawn(f)
        NetworkService {
            addr: socket,
            tx,

        }
    }
}

impl AsRef<SocketAddr> for <State> NetworkService<State> {
    fn as_ref(&self) -> &T {
        
    }
}