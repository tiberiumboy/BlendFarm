use super::behaviour::BlendFarmBehaviour;
use super::message::{Command, NetEvent, NetworkError};
use crate::models::behaviour::BlendFarmBehaviourEvent;
use crate::models::computer_spec::ComputerSpec;
use libp2p::futures::channel::oneshot;
use libp2p::futures::StreamExt;
use libp2p::request_response::OutboundRequestId;
use libp2p::{
    gossipsub::{self, IdentTopic},
    identity, kad, mdns,
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::{io, join, select};

/*
the idea behind this is to have a persistence model to contain network services.
TODO: Find a way to send notification to Tauri application on network process message.
TODO: obtain basic computer specs (CPU/GPU/RAM/OS/Arch/Blender installed)
*/

pub type Port = u16;

pub struct NetEventLoop {
    swarm: Swarm<BlendFarmBehaviour>,
    cmd_recv: Receiver<Command>,
    event_sender: Sender<NetEvent>,
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
    // pending_start_receiving: HashMap<kad::QueryId, oneshot::Sender<()>>,
    // pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
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
            // pending_start_receiving: Default::default(),
            // pending_get_providers: Default::default(),
            pending_request_file: Default::default(),
        }
    }

    pub async fn run(mut self) {
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
        match command {
            /*
            Command::StartJob(job) => todo!(),
            Command::FrameCompleted(path_buf, _) => todo!(),
            Command::EndJob { job_id } => todo!(),
            Command::Status(_) => todo!(),
            Command::RequestFile {
                file_name,
                peer,
                sender,
            } => todo!(),
            Command::RespondFile { file, channel } => todo!(),
            */
            _ => println!("Received commands: {command:?}"),
        }
    }
}

pub struct Host {
    receiver: Receiver<Command>,
    net_service: NetworkService,
}

impl Host {
    pub fn new(net_service: NetworkService, receiver: Receiver<Command>) -> Self {
        Self {
            receiver,
            net_service,
        }
    }

    pub async fn run(&mut self, app_handle: Arc<RwLock<AppHandle>>) {
        loop {
            select! {
                Some(msg) = self.receiver.recv() => {
                    if let Err(e) = self.net_service.send(msg).await {
                        println!("Fail to send net service message: {e:?}");
                    }
                }
                Some(info) = self.net_service.rx_recv.recv() => match info {
                    NetEvent::Render(job) => println!("Job: {job:?}"),
                    NetEvent::Status(msg) => println!("Status: {msg:?}"),
                    NetEvent::NodeDiscovered(peer_id) => {
                        let handle = app_handle.read().unwrap();
                        handle.emit("node_discover", &peer_id).unwrap();
                        println!("Sending identity");
                        let result = self.net_service.tx.send(Command::SendIdentity { peer_id });
                        let _ = join!(result);
                    },
                    NetEvent::NodeDisconnected(peer_id) => {
                        let handle = app_handle.read().unwrap();
                        handle.emit("node_disconnect", peer_id).unwrap();
                    },
                    NetEvent::Identity{peer_id, comp_spec} => {
                        let handle = app_handle.read().unwrap();
                        handle.emit("node_identity", (peer_id, comp_spec)).unwrap();
                    }
                }
            }
        }
    }
}

// this will help launch libp2p network. Should use QUIC whenever possible!
#[derive(Debug)] // at the most, it should implement debug trait. Not anything else!
pub struct NetworkService {
    tx: Sender<Command>,
    pub rx_recv: Receiver<NetEvent>,
    task: JoinHandle<()>,
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
                            Command::StartJob(job) => NetEvent::Render(job).ser(),
                            // Send message to all other peer to stop the target job ID and remove from kad provider
                            Command::EndJob { .. } => todo!(),
                            // send status update
                            Command::Status(s) => NetEvent::Status(s).ser(),

                            // TODO: For Future impl. See how we can transfer the file using kad's behaviour (DHT)
                            Command::RequestFile { .. } => todo!(),
                            Command::RespondFile { .. } => todo!(),
                            Command::SendIdentity { peer_id } => NetEvent::Identity { peer_id, comp_spec: ComputerSpec::default() }.ser(),
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
                                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                                // TODO: Get the computer information and send it to the connector.
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
