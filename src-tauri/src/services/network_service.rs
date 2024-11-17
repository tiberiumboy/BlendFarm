use super::client;
/*

the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.

*/
use super::message::{CmdCommand, FromNetwork, NetMessage, ToNetwork};
use super::{client::Client, server::Server};
use crate::models::render_node::RenderNode;
use local_ip_address::local_ip;
use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeEvent, StoredNetEvent, StoredNodeEvent};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use thiserror::Error;

// Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
// pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";
pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
pub const MULTICAST_PORT: u16 = 3010;
pub const MULTICAST_SOCK: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_ADDR), MULTICAST_PORT);
pub const CHUNK_SIZE: usize = 65536;

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

pub struct NetworkService {
    // I feel like there's a better way to do this
    server: Option<Arc<RwLock<Server>>>,
    client: Arc<RwLock<Client>>,
    tx: Sender<ToNetwork>,
    pub rx_recv: Arc<Receiver<CmdCommand>>,
}

impl NetworkService {
    pub fn new(is_hosting: bool) -> Self {
        let (handler, listener) = node::split::<FromNetwork>();

        let (task, mut receiver) = listener.enqueue();

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

        // TODO: Load from user_settings.rs
        let port = 15000;
        let nodes = Vec::new();
        // use mpsc here
        let (tx, rx) = mpsc::channel();
        let server = match is_hosting {
            true => {
                let server = Server::new(port);
                Some(Arc::new(RwLock::new(server)))
            }
            false => None,
        };

        let client = Client::new();
        client.ping();
        let client = Arc::new(RwLock::new(client));

        let (self_client, self_server) = (client.clone(), server.clone());

        let (_task, mut rx_recv) = mpsc::channel();

        // this seems dangerous - how do we handle this spawn child?
        thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        ToNetwork::Connect(addr) => {
                            if let Some(server) = self_server {
                                server.read().unwrap().connect_peer(addr);
                            } else {
                                self_client.write().unwrap().connect(addr);
                            }
                        }
                        ToNetwork::Ping => {
                            let addr = match self_server {
                                Some(server) => server.read().unwrap().as_ref(),
                                None => self_client.read().unwrap().as_ref(),
                            };
                            let ping = UdpMessage::Ping {
                                host: self_server.is_some(),
                                addr: addr.clone(),
                                name: "Render Node".to_owned(), // TODO: Change this to reflect current host machine name
                            };
                            handler.network().send(udp_conn, &ping.ser());
                        }
                    }

                    if let Some(NetEvent::Network(event)) = receiver.try_receive() {
                        match event {
                            StoredNetEvent::Message(endpoint, bytes) => {
                                let msg = match UdpMessage::de(&bytes) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!("Error deserializing net message data! \n{e}");
                                        continue;
                                    }
                                };

                                match msg {
                                    UdpMessage::Ping { host, addr, name } if host == false => {
                                        if let Some(server) = self_server {
                                            let mut server = server.write().unwrap();
                                            server.connect_peer(addr);
                                        }
                                    }
                                    UdpMessage::Pign { host, addr, name } if host {
                                        if self_client.is_connected() {
                                            todo!();
                                        }
                                    }
                                }
                            }
                            StoredNetEvent::Accepted(_, _) => {}
                            StoredNetEvent::Connected(_, _) => {}
                            StoredNetEvent::Disconnected(_) => {} // NodeEvent::Signal(signal) => {
                                                            //     signal::Stream(data: Option<(Vec<u8>, usize, usize)) => {

                                                            //     }
                                                            // }
                        }
                    }
                }
            }
        });

        let addr = match server {
            Some(server) => server.read().unwrap().as_ref(),
            None => client.read().unwrap().as_ref(),
        };
        let localhost = RenderNode::new("local".to_string(), addr.clone());

        let nodes = vec![localhost];
        Self {
            server,
            client,
            tx,
            rx_recv: Arc::new(rx_recv),
        }
    }

    pub fn add_peer(&self, socket: SocketAddr) {
        // this concerns me. if I take it out, does that mean the option is none?
        if let Some(server) = self.server.clone() {
            let server = server.read().unwrap();
            server.connect_peer(socket);
        }
    }

    pub fn ping(&self, addr: SocketAddr) {}

    // feels like this should be async? Is it not?
    pub fn send_file(&self, file: PathBuf) -> Result<bool, NetworkError> {
        if let Some(server) = self.server.clone() {
            let server = server.read().unwrap();
            server.send_file(file);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
