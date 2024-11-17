/*
the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.

*/
use super::message::{FromNetwork, ToNetwork};
use super::{client::Client, server::Server};
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

pub trait NetworkNode : Send + Sync {
    // fn receiver(&self) -> &Arc<mpsc::Receiver<NetResponse>>;
    fn ping(&self);
    fn send_file(&self, file: PathBuf);
}

pub struct NetworkService {
    // I feel like there's a better way to do this
    // server: Option<Arc<RwLock<Server>>>,
    // client: Arc<RwLock<Client>>,
    connection: Arc<RwLock<dyn NetworkNode>>,
    _tx: Sender<ToNetwork>,
    // pub rx_recv: Arc<Receiver<CmdCommand>>,
}

impl NetworkService {
    pub fn new(is_hosting: bool) -> Self {
        let (handler, listener) = node::split::<FromNetwork>();

        let (_task, mut receiver) = listener.enqueue();

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
        // use mpsc here
        let (tx, rx) = mpsc::channel();
        let server = match is_hosting {
            true => {
                let server = Server::new(port);
                Some(Arc::new(RwLock::new(server)))
            }
            false => None,
        };

        let client: Client = Client::new();
        client.ping();
        let client = Arc::new(RwLock::new(client));

        let (self_client, self_server) = (client.clone(), server.clone());

        // let (_task, mut rx_recv) = mpsc::channel();

        // this seems dangerous - how do we handle this spawn child?
        thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        ToNetwork::Connect(addr) => {
                            // if let Some(server) = self_server {
                            //     server.read().unwrap().connect_peer(addr);
                            // } else {
                                self_client.write().unwrap().connect(addr);
                            // }
                        }
                        ToNetwork::Ping => {
                            let addr = &self_client.read().unwrap().as_ref().clone();
                            let ping = UdpMessage::Ping {
                                host: self_server.is_some(),
                                addr: addr.clone(),
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
                            StoredNetEvent::Disconnected(_) => {} // NodeEvent::Signal(signal) => {
                                                            //     signal::Stream(data: Option<(Vec<u8>, usize, usize)) => {

                                                            //     }
                                                            // }
                        }
                    }
                }
            }
        });

        Self {
            // server,
            connection: client,
            _tx: tx,
        }
    }

    pub fn add_peer(&self, _socket: SocketAddr) {
        // this concerns me. if I take it out, does that mean the option is none?
        let _server = self.connection.read().unwrap(); 
        // _server.connect_peer(_socket);
    }

    pub fn ping(&self) {
        let client = self.connection.read().unwrap();
        client.ping();
    }

    pub fn send_file(&self, file: PathBuf) -> Result<bool, NetworkError> {
        let server = self.connection.read().unwrap();
        server.send_file(file);
        Ok(true)
    }
}
