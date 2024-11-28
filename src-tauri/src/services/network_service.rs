use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{identity, kad, mdns, Multiaddr, PeerId};
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
    // Used as a DHT (BitTorrent) for file transfer
    kad: kad::Behaviour<kad::store::MemoryStore>, // Ok so I need to figure out how this works? Figure out about TStore trait
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
    NodeDiscovered(String),
    NodeDisconnected(String),
}

impl NetMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

pub struct NetEventLoop {
    // message_id: MessageId,
    // cmd: Sender<UiMessage>,
    // response: Receiver<NetMessage>,
    // _task: JoinHandle<()>,
}

// TODO: finish implementation for this.
impl NetEventLoop {
    // pub fn new(peer_id: &PeerId, cmd: Sender<UiMessage>, response: Receiver<NetMessage>) -> Self {
    //     message_id: MessageId::new(peer_id.to_bytes()),
    //     cmd,
    //     response,
    //     _task = tokio::spawn( )
    //     }
    // }
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

        // TODO: How come no other swarm can see the topic?
        let topic = IdentTopic::new("blendfarm-rpc-msg");

        let peer_id = identity::Keypair::generate_ed25519();

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

                // used as a DHT - can be a provider or a client.
                // TODO: need to read the documentation for more information about kademline implementation. (It acts like bitTorrent)
                let kad = kad::Behaviour::new(
                    peer_id.public().to_peer_id(),
                    kad::store::MemoryStore::new(key.public().to_peer_id()),
                );

                Ok(BlendFarmBehaviour {
                    gossipsub,
                    mdns,
                    kad,
                })
            })
            .expect("Expect to build behaviour")
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
            .build();

        match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
            Ok(_) => println!("Successully subscribed!"),
            Err(e) => println!("Fail to subscribe topic! {e:?}"),
        };

        // TODO: Find a way to fetch user configuration. Refactor this when possible.
        let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
            .parse()
            .expect("Must be valid multiaddr");

        let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("Must be valid multiaddr");

        swarm.listen_on(tcp).expect("Fail to listen on TCP");
        swarm.listen_on(udp.clone()).expect("Fail to listen on UDP");

        let _ = swarm.dial(PeerId::random());

        // create a new channel with a capacity of at most 32.
        let (tx, mut rx) = mpsc::channel::<UiMessage>(32);

        // create a new receiver from the network stack of at most 32.
        let (tx_recv, rx_recv) = mpsc::channel::<NetMessage>(32);

        // Create a new object for this struct handler NetEventLoop?
        let _task = tokio::spawn(async move {
            loop {
                select! {
                    // Sender
                    Some(signal) = rx.recv() => {
                        let data = signal.ser();
                        println!("sending message to gossip {:?}", &signal);
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), data) {
                            // Ok(msg_id) => msg_id,
                            // Err(e) => {
                                println!("Fail to publish message to swarm! {e:?}");
                                // MessageId::

                        }
                    }

                    // Receive
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {

                            // so the list contains all of the peer_id information?
                            for (peer_id, .. ) in list {
                                println!("mDNS discovered a new peer: {}", &peer_id);
                                if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                                    println!("{e:?}");
                                };
                                if let Err(e) = tx_recv.send(NetMessage::NodeDiscovered(peer_id.to_string())).await {
                                    println!("Error sending node discovered {e:?}");
                                }
                                 // let's do a test here then?
                                {
                            //     swarm.behaviour_mut().gossipsub.publish()
                            //         let _ = server
                            //             .to_network
                            //             .send(UiMessage::Status("Hello world!".to_owned()))
                            //             .await;
                                }
                            }
                        }
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: id,
                            message,
                        })) => {
                            if let Ok(msg) = NetMessage::de(&message.data) {
                                println!("Got message: '{msg:?}' with id: {id} from peer: {peer_id}");
                                let _ = tx_recv.send(msg).await;
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

    // TODO: find a way to add to poll?
    pub async fn send(&self, msg: UiMessage) -> Result<(), NetworkError> {
        self.tx
            .send(msg)
            .await
            .map_err(|e| NetworkError::SendError(e.to_string()))
    }
}

impl AsRef<JoinHandle<()>> for NetworkService {
    fn as_ref(&self) -> &JoinHandle<()> {
        &self.task
    }
}
