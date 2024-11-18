use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{mdns, Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::{io, select};
use uuid::Uuid;

use crate::models::job::Job;

/*
the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.
TODO: obtain basic computer specs (CPU/GPU/RAM/OS/Arch/Blender installed)

Resources:
    Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
*/

pub type Port = u16;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Not Connected")]
    NotConnected,
    #[error("Invalid connection")]
    Invalid,
    #[error("Bad Input")]
    BadInput,
    #[error("Send Error: {0}")]
    SendError(String),
}

// TODO: Extract this into separate file
#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
    // ping: libp2p::ping::Behaviour,
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    // I want to include kad for file share protocol
    // kad: libp2p::kad::Behaviour<T>// Ok so I need to figure out how this works? Figure out about TStore trait
}

// TODO: Extract this into separate file
#[derive(Debug, Serialize, Deserialize)]
pub enum UiMessage {
    SendFile(PathBuf),
    StartJob(Job),
    EndJob { job_id: Uuid },
    Status(String),
}

impl UiMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

// TODO: Extract this into separate file
#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    // Greet () // share machine spec (cpu, gpu, ram)
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM usage realtime)
    Render(Job),
    // think I need to send this somewhere else.
    // SendFile { file_name: String, data: Vec<u8> },
    Status(String),
}

impl NetMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

// this will help launch libp2p network. Should use QUIC whenever possible!
pub struct NetworkService {
    tx: Sender<UiMessage>,
    pub rx_recv: Receiver<NetMessage>,
    task: JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum Networkerror {
    #[error("Unexpected error")]
    Unexpected,
}

impl NetworkService {
    // I think it's best if we take in common demoniator value as possible.
    pub async fn new(idle_connection_timeout: u64) -> Result<Self, NetworkError> {
        // try to parse the duration before usage.
        let duration = Duration::from_secs(idle_connection_timeout);
        // required by swarm
        let tcp_config: libp2p::tcp::Config = libp2p::tcp::Config::default();
        // TODO: Figure out what this one is suppose to do?
        let topic = IdentTopic::new("blendfarm-rpc-msg");

        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp_config,
                libp2p::tls::Config::new,
                libp2p::yamux::Config::default,
            )
            .expect("Should be able to build with tcp configuration?")
            .with_quic()
            .with_behaviour(|key| {
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s); // what was this suppose to do?
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .message_id_fn(message_id_fn)
                    .build()
                    .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .expect("Fail to create gossipsub behaviour");

                let mdns =
                    mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                        .expect("Fail to create mdns behaviour!");

                Ok(BlendFarmBehaviour { gossipsub, mdns })
            })
            .expect("Expect to build behaviour")
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
            .build();

        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .expect("Should be able to subscribe");

        // TODO: Find a way to fetch user configuration. Refactor this when possible.
        let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
            .parse()
            .expect("Must be valid multiaddr");
        let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("Must be valid multiaddr");
        swarm.listen_on(tcp).expect("Fail to listen on TCP");
        swarm.listen_on(udp).expect("Fail to listen on UDP");

        // swarm.dial(udp);
        swarm.dial(PeerId::random());

        // create a new channel with a capacity of at most 32.
        let (tx, mut rx) = mpsc::channel::<UiMessage>(32);

        // create a new receiver from the network stack of at most 32.
        let (tx_recv, rx_recv) = mpsc::channel::<NetMessage>(32);

        // create a thread here
        let _task = tokio::spawn(async move {
            loop {
                select! {
                    // Sender
                    Some(signal) = rx.recv() => {
                        let topic = gossipsub::IdentTopic::new("blendfarm-rpc-msg");
                        let data = signal.ser();
                        println!("sending message to gossip {:?}", &data);
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                            println!("Fail to publish message to swarm! {e:?}");
                        }
                    }

                    // Receive
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            // it would be nice to show the list of user to the UI?
                            for (peer_id, .. ) in list {
                                println!("mDNS discovered a new peer: {}", &peer_id);
                                // tx_recv.send(NetMessage) // Could implement a new notification on peer connected?
                            }
                        }
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: id,
                            message,
                        })) => {
                            if let Ok(msg) = NetMessage::de(&message.data) {
                                println!("Got message: '{msg:?}' with id: {id} from peer: {peer_id}");
                                tx_recv.send(msg).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(Self {
            tx,
            rx_recv,
            task: _task,
        })
    }

    pub async fn send(&self, msg: UiMessage) -> Result<(), NetworkError> {
        self.tx
            .send(msg)
            .await
            .map_err(|e| NetworkError::SendError(e.to_string()))
    }

    // pub async fn send_status(&mut self, status: String) -> Result<(), NetworkError> {
    //     let msg = UiMessage::Status(status);
    //     let _ = self.tx.send(msg).await;
    //     Ok(())
    // }

    // pub async fn init_distribute_job(&mut self, job: Job) -> Result<(), NetworkError> {
    //     // here we will peek at the job and see if it's a frame or a window. If it's a frame, then we could sub divide the task to render segment instead?
    //     let msg = UiMessage::StartJob(job);
    //     let _ = self.tx.send(msg).await;
    //     Ok(())
    // }

    // pub async fn stop_distribute_job(&mut self, job_id: Uuid) -> Result<(), NetworkError> {
    //     let msg = UiMessage::EndJob { job_id };
    //     let _ = self.tx.send(msg).await;
    //     Ok(())
    // }

    // pub async fn send_file(&mut self, file_path: &impl AsRef<Path>) -> Result<(), NetworkError> {
    // ok how can I transfer file here?
    //     let msg = UiMessage::SendFile(file_path.as_ref().to_path_buf());
    //     let _ = self.tx.send(msg).await;
    //     Ok(())
    // }
}

impl AsRef<JoinHandle<()>> for NetworkService {
    fn as_ref(&self) -> &JoinHandle<()> {
        &self.task
    }
}

impl NetworkService<Online> {}

impl NetworkService {
    pub fn new(is_hosting: bool) -> Self {
        let (handler, listener) = node::split::<FromNetwork>();
        
        /* 
        // something about this line takes forever to process and load?
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

        // TODO: Load from user_settings.rs
        let port = 15000;
        let nodes = Vec::new();
        let addr;
        // use mpsc here
        let (tx, rx) = mpsc::channel();
        let (server, client) = match is_hosting {
            true => {
                addr = SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), port);
                let server = Server::new(addr);
                {}

                (Some(Arc::new(RwLock::new(server))), None)
            }
            false => {
                addr = SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), 0);
                let client = Client::new(addr);
                client.ping();
                (None, Some(Arc::new(RwLock::new(client))))
            }
        };

        let client = Client::new(addr);
        client.ping();
        let client = Arc::new(RwLock::new(client));

        let (self_client, self_server) = (client.clone(), server.clone());

        // let (_task, mut rx_recv) = mpsc::channel();

        // this seems dangerous - how do we handle this spawn child?
        thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                /*                 if let Ok(msg) = rx.try_recv() {
                    match msg {
                        ToNetwork::Connect(addr) => {
                            // if let Some(server) = self_server {
                            //     server.read().unwrap().connect_peer(addr);
                            // } else {
                            self_client.write().unwrap().connect(addr);
                            // }
                        } // UdpMessage::Ping { host, addr, name } if host => {
                          //     // an attempt to connect
                          //     if self_server.is_none() {
                          //         let service = self_client.write().unwrap();
                          //         service.connect(addr);
                          //     }
                          // },
                          // UdpMessage::Ping { addr, name, host } => {
                          //     if let Some(server) = self_server {
                          //         let service = server.write().unwrap();
                          //         service.connect_peer(addr);
                          //     }
                          // },
                          // };
                    }
                };
                

                    if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                        match event {
                            NetEvent::Message(endpoint, bytes) => {
                                let msg = match UdpMessage::deserialize(&bytes) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!("Error deserializing net message data! \n{e}");
                                        continue;
                                    }
                                };

                                match msg {
                                    UdpMessage::Ping { host, addr, name } => {
                                        if let Some(server) = self_server {
                                            let mut server = server.write().unwrap();
                                            server.ping();
                                        }

                                        let client = self_client.read().unwrap();
                                        client.ping();
                                    }
                                }
                            }
                            NetEvent::Accepted(_, _) => {}
                            NetEvent::Connected(_, _) => {}
                            NetEvent::Disconnected(_) => {} // NodeEvent::Signal(signal) => {
                                                            //     signal::Stream(data: Option<(Vec<u8>, usize, usize)) => {

                                                            //     }
                                                            // }
                        }
                    }
                }
            }
        });

        let localhost = RenderNode::new("local".to_string(), addr);

        let nodes = vec![localhost];
        Self {
            server,
            client,
            tx,
            rx_recv: Arc::new(rx_recv),
            addr,
        }
    }

    pub fn add_peer(&self, _socket: SocketAddr) {
        // this concerns me. if I take it out, does that mean the option is none?
        let _server = self.connection.read().unwrap();
        // _server.connect_peer(_socket);
    }

    pub fn ping(&self) {
        self.connection.ping();
    }

    pub fn send_file(&self, file: PathBuf) -> Result<bool, NetworkError> {
        self.connection.send_file(file);
        Ok(true)
    }
}
