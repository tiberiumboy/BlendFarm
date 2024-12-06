use super::behaviour::{BlendFarmBehaviour, FileResponse};
use super::computer_spec::ComputerSpec;
use super::message::{NetCommand, NetEvent, NetworkError};
use crate::models::behaviour::BlendFarmBehaviourEvent;
use futures::channel::oneshot;
use libp2p::futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic},
    identity, kad, mdns,
    swarm::{Swarm, SwarmEvent},
    tcp, Multiaddr, SwarmBuilder,
};
use libp2p::{PeerId, StreamProtocol};
use libp2p_request_response::{OutboundRequestId, ProtocolSupport, ResponseChannel};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;
use std::time::Duration;
use std::u64;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::{io, select};

/*
Network Service - Provides simple network interface for peer-to-peer network for BlendFarm.
Includes mDNS ()
*/

pub static TOPIC: LazyLock<IdentTopic> = LazyLock::new(|| IdentTopic::new("blendfarm-rpc-msg"));

// the tuples return three objects
// the NetworkService holds the network loop operation
// the Network Controller to send command to network service
// the receiver command from network services
pub async fn new() -> Result<(NetworkService, NetworkController, Receiver<NetEvent>), NetworkError>
{
    let duration = Duration::from_secs(u64::MAX);
    let id_keys = identity::Keypair::generate_ed25519();
    let tcp_config: tcp::Config = tcp::Config::default();

    let mut swarm = SwarmBuilder::with_new_identity()
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
                key.public().to_peer_id(),
                kad::store::MemoryStore::new(key.public().to_peer_id()),
            );

            let rr_config = libp2p_request_response::Config::default();
            let protocol = [(
                StreamProtocol::new("/file-exchange/1"),
                ProtocolSupport::Full,
            )];
            let request_response = libp2p_request_response::Behaviour::new(protocol, rr_config);

            Ok(BlendFarmBehaviour {
                request_response,
                gossipsub,
                mdns,
                kad,
            })
        })
        .expect("Expect to build behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
        .build();

    // should move this?
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&TOPIC) {
        println!("Fail to subscribe topic! {e:?}");
    };

    // TODO: Find a way to fetch user configuration. Refactor this when possible.
    let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0"
        .parse()
        .expect("Must be valid multiaddr");

    let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
        .parse()
        .expect("Must be valid multiaddr");

    swarm.listen_on(tcp).expect("Fail to listen on TCP");
    swarm.listen_on(udp.clone()).expect("Fail to listen on UDP");

    if let Err(e) = swarm.dial(id_keys.public().to_peer_id()) {
        eprintln!("Fail to dial swarm with random ID: {e:?}"); // I need to figure out what the error message here?
    }

    // the command sender is used for outside method to send message commands to network queue
    let (command_sender, command_receiver) = mpsc::channel::<NetCommand>(32);
    // the event sender is used to handle incoming network message. E.g. RunJob
    let (event_sender, event_receiver) = mpsc::channel::<NetEvent>(32);

    Ok((
        NetworkService {
            swarm,
            command_receiver,
            event_sender,
            pending_get_providers: Default::default(),
            pending_start_providing: Default::default(),
            pending_request_file: Default::default(),
        },
        NetworkController {
            sender: command_sender,
        },
        event_receiver,
    ))
}

#[derive(Clone)]
pub struct NetworkController {
    sender: mpsc::Sender<NetCommand>,
}

impl NetworkController {
    pub async fn share_computer_info(&mut self) {
        self.sender
            .send(NetCommand::SendIdentity)
            .await
            .expect("Command should not have been dropped");
    }

    pub async fn start_providing(&mut self, file_name: String) {
        let (sender, receiver) = oneshot::channel();
        let cmd = NetCommand::StartProviding { file_name, sender };
        self.sender
            .send(cmd)
            .await
            .expect("Command receiver not to be dropped");
        receiver.await.expect("Sender should not be dropped");
    }

    pub async fn get_providers(&mut self, file_name: String) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::GetProviders { file_name, sender })
            .await
            .expect("Command receiver should not be dropped");
        receiver.await.expect("Sender should not be dropped")
    }

    pub(crate) async fn request_file(
        &mut self,
        peer: PeerId,
        file_name: String,
    ) -> Result<Vec<u8>, Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::RequestFile {
                file_name,
                peer,
                sender,
            })
            .await
            .expect("Command should not be dropped");
        receiver.await.expect("Sender should not be dropped")
    }

    pub(crate) async fn respond_file(
        &mut self,
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    ) {
        self.sender
            .send(NetCommand::RespondFile { file, channel })
            .await
            .expect("Command should not be dropped");
    }
}

// this will help launch libp2p network. Should use QUIC whenever possible!
pub struct NetworkService {
    // swarm behaviour - interface to the network
    swarm: Swarm<BlendFarmBehaviour>,
    // receive Network command
    pub command_receiver: Receiver<NetCommand>,
    // send network events
    event_sender: Sender<NetEvent>,
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_start_providing: HashMap<kad::QueryId, oneshot::Sender<()>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

impl NetworkService {
    // send command
    async fn handle_command(&mut self, cmd: NetCommand) {
        println!("Handle command: {cmd:?}");
        match cmd {
            // Begin the job
            // The idea here is that we received a new job from the host -
            // we would need to upload blender to kad service and make it public available for DHT to access for other nodes to obtain
            // then we send out notification to all of the node to start the job
            NetCommand::StartJob(job) => {
                // NetEvent::Render(job).ser()
                // receives a job request. can do fancy behaviour like split up the job into different frames?
                // todo: For now, send the job request.
                let data = NetEvent::Render(job).ser();
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(TOPIC.clone(), data)
                {
                    eprintln!("Fail to send job! {e:?}");
                }
            }
            // Send message to all other peer to stop the target job ID and remove from kad provider
            NetCommand::EndJob { .. } => todo!(),
            // send status update
            NetCommand::Status(s) => {
                let peer_id = self.swarm.local_peer_id().to_string();
                let data = NetEvent::Status(peer_id, s).ser();
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(TOPIC.clone(), data)
                {
                    eprintln!("Fail to send status over network! {e:?}");
                }
            }

            // TODO: For Future impl. See how we can transfer the file using kad's behaviour (DHT)
            // NetCommand::RequestFile { .. } => todo!(),
            // NetCommand::RespondFile { .. } => todo!(),
            NetCommand::SendIdentity => {
                let peer_id = self.swarm.local_peer_id().to_string();
                let data = NetEvent::Identity(peer_id, ComputerSpec::default()).ser();
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(TOPIC.clone(), data)
                {
                    eprintln!("Fail to publish message to swarm! {e:?}");
                    // return Err(NetworkError::SendError(e.to_string()));
                };
            }
            NetCommand::GetProviders { file_name, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kad
                    .get_providers(file_name.into_bytes().into());
                self.pending_get_providers.insert(query_id, sender);
            }
            _ => {
                todo!("What happen here? {cmd:?}");
            }
        };
    }

    async fn handle_event(&mut self, event: SwarmEvent<BlendFarmBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns)) => {
                self.handle_mdns(mdns).await
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossip)) => {
                self.handle_gossip(gossip).await
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::RequestResponse(
                libp2p_request_response::Event::OutboundFailure {
                    request_id, error, ..
                },
            )) => {
                let _ = self
                    .pending_request_file
                    .remove(&request_id)
                    .expect("Request to is still pending")
                    .send(Err(Box::new(error)));
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::RequestResponse(
                libp2p_request_response::Event::ResponseSent { .. },
            )) => {}
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer_id);

                // here we'll say that the node was disconnected.
                // send a message back to the Ui confirming we discover a node (Use this to populate UI element on the front end facing app)
                // let event = LoopEvent(peer_id, NetEvent::NodeDiscovered);
                let event = NetEvent::NodeDiscovered(peer_id.to_string());
                if let Err(e) = self.event_sender.send(event).await {
                    println!("Error sending node discovered signal to UI: {e:?}");
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .remove_explicit_peer(&peer_id);

                // send a message back to the UI notifying the disconnnection of the node
                // let event = LoopEvent(peer_id, NetEvent::NodeDisconnected);
                let event = NetEvent::NodeDisconnected(peer_id.to_string());
                if let Err(e) = self.event_sender.send(event).await {
                    println!("Error sending node disconnected signal to UI: {e:?}");
                }
            }
            _ => {
                println!("Unhandle swarm behaviour event: {event:?}")
            }
        }
    }

    async fn handle_mdns(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, ..) in list {
                    println!("Peer discovered {}", peer_id.to_string());
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                }
            }
            mdns::Event::Expired(list) => {
                for (peer_id, ..) in list {
                    println!("Peer disconnected {}", peer_id.to_string());
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            } // _ => {}
        };
    }

    async fn handle_gossip(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message { message, .. } => {
                // This message internally is used to share NetEvent across the p2p network.
                if let Ok(msg) = NetEvent::de(&message.data) {
                    println!("Received message {msg:?}");
                    // let event = LoopEvent(propagation_source, msg); // here I'd like to capture the peer_id that sent me the message?
                    // if let Err(e) = self.event_sender.send(event).await {
                    //     eprintln!("Fail to send loop event! {e:?}");
                    // }
                    if let Err(e) = self.event_sender.send(msg).await {
                        eprintln!("Fail to send loop event! {e:?}");
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_kademila(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed {
                id,
                result: kad::QueryResult::StartProviding(_),
                ..
            } => {
                if let Some(sender) = self.pending_start_providing.remove(&id) {
                    let _ = sender.send(());
                }
            }
            kad::Event::OutboundQueryProgressed {
                id,
                result:
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        providers,
                        ..
                    })),
                ..
            } => {
                if let Some(sender) = self.pending_get_providers.remove(&id) {
                    sender.send(providers).expect("Receiver not to be dropped");
                    self.swarm
                        .behaviour_mut()
                        .kad
                        .query_mut(&id)
                        .unwrap()
                        .finish();
                }
            }
            kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                    )),
                ..
            } => {}
            _ => {}
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                signal = self.command_receiver.recv() => match signal {
                    Some(command) => self.handle_command(command).await,
                    None => break,
                }
            }
        }
    }
}

impl AsRef<Receiver<NetCommand>> for NetworkService {
    fn as_ref(&self) -> &Receiver<NetCommand> {
        &self.command_receiver
    }
}
