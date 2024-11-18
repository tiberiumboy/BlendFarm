/*
the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.

*/
use super::message::{FromNetwork, ToNetwork};
use local_ip_address::local_ip;
use message_io::network::Transport;
use message_io::node::{self,  StoredNetEvent, StoredNodeEvent};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use thiserror::Error;

// Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
// pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";
pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
pub const MULTICAST_PORT: u16 = 3010;
pub const MULTICAST_SOCK: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_ADDR), MULTICAST_PORT);
pub const CHUNK_SIZE: usize = 65536;

pub type Port = u16;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Not Connected")]
    NotConnected,
    #[error("Invalid connection")]
    Invalid,
    #[error("Bad Input")]
    BadInput,
}

#[derive(Serialize, Deserialize)]
pub enum UdpMessage {
    Ping {
        host: bool,
        addr: SocketAddr,
        name: String,
    },
}

impl UdpMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(&data)
    }
}

pub struct NetworkNode {
    addr: SocketAddr,
    tx: Sender<ToNetwork>,
}

impl NetworkNode {
    pub fn new(port: Port) -> Self {

        let public_addr = SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), port);
        
        let (handler, listener) = node::split::<FromNetwork>();
        
        /* 
        // something about this code blocked main thread from running application.
        let (_task, mut receiver) = listener.enqueue();
        */

        // listen udp
        handler
            .network()
            // could also use user configuration for this one too?
            .listen(Transport::Udp, MULTICAST_SOCK)
            .unwrap();
        
        // connect udp
        let (udp_conn, _) = match handler.network().connect(Transport::Udp, MULTICAST_SOCK) {
            Ok(data) => data,
            // Err(e) if e.kind() == std::io::ErrorKind::NetworkUnreachable => {
            //     todo!("how can we gracefully tell the program to continue to run on local network - despite we're \"offline\"?");
            // }
            Err(e) => panic!("{e}"),
        };

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                                 
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        ToNetwork::Connect(addr) => {
                            handler.network().connect(Transport::FramedTcp, addr).unwrap();
                        }
                        ToNetwork::Ping { host } => {
                            let ping = UdpMessage::Ping {
                                host,
                                addr: public_addr.clone(),
                                name: "Render Node".to_owned(), // TODO: Change this to reflect current host machine name
                            };
                            handler.network().send(udp_conn, &ping.ser());
                        },
                        ToNetwork::SendFile(_file) => {
                            // hmm...?
                        },
                        ToNetwork::Exit => {
                            handler.stop();
                        }
                    }
                };
                
                /*
                if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                    match event {
                        StoredNetEvent::Message(endpoint, bytes) => {
                            println!("Message received from [{}]", endpoint.addr());
                            let msg = match UdpMessage::de(&bytes) {
                                Ok(data) => data,
                                Err(e) => {
                                    println!("Error deserializing net message data! \n{e}");
                                    continue;
                                }
                            };

                            match msg {
                                UdpMessage::Ping { host, addr, name } if host == false => {
                                    println!("UDP Ping intercepted by '{}'[{}]", name, addr);
                                    // if let Some(server) = self_server {
                                    //     let mut server = server.write().unwrap();
                                    //     server.connect_peer(addr);
                                    // }
                                },
                                UdpMessage::Ping { .. } => {
                                    if self_client.read().unwrap().is_connected() {
                                        todo!();
                                    }
                                },
                            }
                        }
                        StoredNetEvent::Accepted(_, _) => {}
                        StoredNetEvent::Connected(_, _) => {}
                        StoredNetEvent::Disconnected(_) => {}
                    }
                }
                */
            }
        });
        
        Self {
            addr: public_addr,
            tx,
        }
    }

    pub fn ping(&self, host: bool) {
        // TODO: Find out how I can make this call panic. whether I need to handle via error handling situation?
        self.tx.send(ToNetwork::Ping { host }).unwrap();
    }

    pub fn send_file(&self, file: PathBuf) {
        // TODO: Find out how I can make this call panic. whether I need to handle via error handling situation?
        self.tx.send(ToNetwork::SendFile(file)).unwrap();
    }
}

impl AsRef<SocketAddr> for NetworkNode {
    fn as_ref(&self) -> &SocketAddr {
        &self.addr
    }
}

pub struct NetworkService {
    connection: NetworkNode,
    is_host: bool,
}

impl NetworkService {
    pub fn new(is_host: bool, port: Port) -> Self {
        // use mpsc here
        // let (tx, rx) = mpsc::channel();
        // let server = match is_host {
        //     true => {
        //         let server = Server::new(port);
        //         Some(Arc::new(RwLock::new(server)))
        //     }
        //     false => None,
        // };

        // let (_task, mut rx_recv) = mpsc::channel();

        // client port can be assigned to any, but host must have a valid port
        let port = if is_host {
            port
        } else {
            0
        };

        Self {
            connection: NetworkNode::new(port),
            is_host
        }
    }

    pub fn add_peer(&self, _socket: SocketAddr) {
        // self.connection.; 
        // _server.connect_peer(_socket);
    }

    pub fn ping(&self) {
        self.connection.ping(self.is_host);
    }

    pub fn send_file(&self, file: PathBuf) -> Result<bool, NetworkError> {
        self.connection.send_file(file);
        Ok(true)
    }
}
