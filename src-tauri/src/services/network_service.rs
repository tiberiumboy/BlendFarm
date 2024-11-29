use libp2p::futures::channel::oneshot;
use libp2p::futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic},
    identity, kad, mdns,
    request_response::{self, ResponseChannel},
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileRequest(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileResponse(Vec<u8>);

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
#[derive(Debug)]
pub enum Command {
    StartJob(Job),
    FrameCompleted(PathBuf, i32),
    EndJob {
        job_id: Uuid,
    },
    Status(String),
    RequestFile {
        file_name: String,
        peer: PeerId,
        sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    },
    RespondFile {
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    },
}

// TODO: Extract this into separate file
#[derive(Debug, Serialize, Deserialize)]
pub enum NetEvent {
    // Greet () // share machine spec (cpu, gpu, ram)
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM usage realtime)
    Render(Job),
    // think I need to send this somewhere else.
    // SendFile { file_name: String, data: Vec<u8> },
    Status(String),
    NodeDiscovered(String),
    NodeDisconnected(String),
}

impl NetEvent {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

// TODO: Use this struct once we can establish communication across network
/*
pub struct NetEventLoop {
    swarm: Swarm<BlendFarmBehaviour>,
    cmd_recv: Receiver<Command>,
    event_sender: Sender<NetEvent>,
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
    // message_id: MessageId,
    // cmd: Sender<UiMessage>,
    // response: Receiver<NetMessage>,
    // _task: JoinHandle<()>,
}

// TODO: finish implementation for this.
impl NetEventLoop {
    pub fn new(
        swarm: Swarm<BlendFarmBehaviour>,
        cmd_recv: Receiver<Command>,
        event_sender: Sender<NetEvent>,
    ) -> Self {
        Self {
            swarm,
            cmd_recv,
            event_sender,
            pending_dial: Default::default(),
        }
    }

    pub(crate) async fn run(mut self) {
        loop {
            select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                Some(command) = self.cmd_recv.recv() => self.handle_command(command).await,
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<BlendFarmBehaviourEvent>) {
        match event {
            SwarmEvent::IncomingConnection { .. } => {}
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        let _ = sender.send(Ok(()));
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_command(&mut self, command: Command) {
        // match command {
        //     UiMessage::
        // }
        println!("Handling command: {command:?}");
    }
}
*/

// this will help launch libp2p network. Should use QUIC whenever possible!
pub struct NetworkService {
    tx: Sender<Command>,
    pub rx_recv: Receiver<NetEvent>,
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
                    message.data.hash(&mut s);
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

                /*
                    Used as a DHT - can be a provider or a client.
                    TODO: Future Impl. need to read the documentation for more information about kademline implementation. (It acts like bitTorrent)
                        We will be using this for file transfer for both blend files, blender version, and render image result.
                        For now we're going to use
                */
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

        let topic = IdentTopic::new("blendfarm-rpc-msg");

        if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
            println!("Fail to subscribe topic! {e:?}");
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

        // TODO: Future impl. Make the size of buffer adjustable by user configuration
        // create a new channel with a capacity of at most 32 message max.
        let (tx, mut rx) = mpsc::channel::<Command>(32);

        // create a new receiver from the network stack of at most 32 message max.
        let (tx_recv, rx_recv) = mpsc::channel::<NetEvent>(32);

        // Create a new object to hold this info - use NetEventLoop?
        let _task = tokio::spawn(async move {
            loop {
                select! {
                    // TODO: Extrapolate this out into a separate struct implementation to make it maintainable and easy to read
                    // Sender
                    Some(signal) = rx.recv() => {
                        // println!("received command {:?}", &signal);
                        let data = match signal {
                            // Begin the job
                            // The idea here is that we received a new job from the host -
                            // we would need to upload blender to kad service and make it public available for DHT to access for other nodes to obtain
                            // then we send out notification to all of the node to start the job
                            Command::StartJob(_) => todo!(),
                            // Send message to all other peer to stop the target job ID and remove from kad provider
                            Command::EndJob { .. } => todo!(),
                            // send status update
                            Command::Status(s) => NetEvent::Status(s).ser(),

                            // TODO: For Future impl. See how we can transfer the file using kad's behaviour (DHT)
                            Command::RequestFile { .. } => todo!(),
                            Command::RespondFile { .. } => todo!(),
                            _ => {
                                return;
                            }
                        };

                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), data) {
                            println!("Fail to publish message to swarm! {e:?}");
                        }
                    }

                    // Receive
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, .. ) in list {
                                println!("mDNS discovered a new peer: {}", &peer_id);
                                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                                // send a message back to the Ui confirming we discover a node (Use this to populate UI element on the front end facing app)
                                if let Err(e) = tx_recv.send(NetEvent::NodeDiscovered(peer_id.to_string())).await {
                                    println!("Error sending node discovered signal to UI{e:?}");
                                }
                            }
                        }
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                            for(peer_id, .. ) in list {
                                println!("mDNS discover peer has disconnected: {peer_id}");
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);

                                // send a message back to the UI notifying the disconnnection of the node
                                if let Err(e) = tx_recv.send(NetEvent::NodeDisconnected(peer_id.to_string())).await {
                                    println!("Error sending node disconnected signal to UI: {e:?}");
                                }
                            }
                        }
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: id,
                            message,
                        })) => {
                            // This message internally is used to share NetEvent across the p2p network.
                            if let Ok(msg) = NetEvent::de(&message.data) {
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
    pub async fn send(&self, msg: Command) -> Result<(), NetworkError> {
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
