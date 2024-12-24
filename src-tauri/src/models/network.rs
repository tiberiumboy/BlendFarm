use super::behaviour::{BlendFarmBehaviour, FileRequest, FileResponse};
use super::computer_spec::ComputerSpec;
use super::job::JobEvent;
use super::message::{NetCommand, NetEvent, NetworkError};
use super::server_setting::ServerSetting;
use crate::models::behaviour::BlendFarmBehaviourEvent;
use core::str;
use futures::{channel::oneshot, prelude::*, StreamExt};
use libp2p::{
    ping,
    gossipsub::{self, IdentTopic},
    kad, mdns,
    swarm::{Swarm, SwarmEvent},
    tcp, Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use libp2p_request_response::{OutboundRequestId, ProtocolSupport, ResponseChannel};
use machine_info::Machine;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use std::u64;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::{io, select};

/*
Network Service - Provides simple network interface for peer-to-peer network for BlendFarm.
Includes mDNS ()
*/

pub const STATUS: &str = "blendfarm/status";
pub const SPEC: &str = "blendfarm/spec";
pub const JOB: &str = "blendfarm/job";
pub const HEARTBEAT: &str = "blendfarm/heartbeat";
const TRANSFER: &str = "/file-transfer/1";

// the tuples return three objects
// the NetworkService holds the network loop operation
// the Network Controller to send command to network service
// the Receiver<NetCommand> from network services
pub async fn new() -> Result<(NetworkService, NetworkController, Receiver<NetEvent>), NetworkError>
{
    let duration = Duration::from_secs(u64::MAX);
    // let id_keys = identity::Keypair::generate_ed25519();
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

            let ping_config = ping::Config::default();
            let ping = ping::Behaviour::new(ping_config);  

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                // .validation_mode(gossipsub::ValidationMode::Strict)
                // .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

            // p2p communication
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .expect("Fail to create gossipsub behaviour");

            // network discovery usage
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                    .expect("Fail to create mdns behaviour!");

            // Used to provide file provision list
            let kad = kad::Behaviour::new(
                key.public().to_peer_id(),
                kad::store::MemoryStore::new(key.public().to_peer_id()),
            );

            let rr_config = libp2p_request_response::Config::default();
            let protocol = [(StreamProtocol::new(TRANSFER), ProtocolSupport::Full)];
            let request_response = libp2p_request_response::Behaviour::new(protocol, rr_config);

            Ok(BlendFarmBehaviour {
                ping,
                request_response,
                gossipsub,
                mdns,
                kad,
            })
        })
        .expect("Expect to build behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
        .build();

    let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0"
        .parse()
        .map_err(|_| NetworkError::BadInput)?;

    let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
        .parse()
        .map_err(|_| NetworkError::BadInput)?;

    swarm
        .listen_on(tcp)
        .map_err(|e| NetworkError::UnableToListen(e.to_string()))?;
    swarm
        .listen_on(udp)
        .map_err(|e| NetworkError::UnableToListen(e.to_string()))?;

    swarm.behaviour_mut().kad.set_mode(Some(kad::Mode::Server));

    // the command sender is used for outside method to send message commands to network queue
    let (command_sender, command_receiver) = mpsc::channel::<NetCommand>(32);
    // the event sender is used to handle incoming network message. E.g. RunJob
    let (event_sender, event_receiver) = mpsc::channel::<NetEvent>(32);

    Ok((
        NetworkService {
            swarm,
            command_receiver,
            event_sender,
            machine: Machine::new(),
            pending_get_providers: Default::default(),
            pending_start_providing: Default::default(),
            pending_request_file: Default::default(),
        },
        NetworkController {
            sender: command_sender,
            settings: ServerSetting::load(),
            providing_files: Default::default(),
        },
        event_receiver,
    ))
}

#[derive(Clone)]
pub struct NetworkController {
    sender: mpsc::Sender<NetCommand>,
    pub settings: ServerSetting,
    pub providing_files: HashMap<String, PathBuf>,
}

impl NetworkController {
    pub async fn subscribe_to_topic(&mut self, topic: String) {
        self.sender
            .send(NetCommand::SubscribeTopic(topic))
            .await
            .unwrap();
    }

    pub async fn unsubscribe_from_topic(&mut self, topic: String) {
        self.sender
            .send(NetCommand::UnsubscribeTopic(topic))
            .await
            .unwrap();
    }

    pub async fn send_status(&mut self, status: String) {
        println!("[Status]: {status}");
        self.sender
            .send(NetCommand::Status(status))
            .await
            .expect("Command should not been dropped");
    }

    // How do I get the peers info I want to communicate with?
    pub async fn send_job_message(&mut self, target: PeerId, event: JobEvent) {
        self.sender
            .send(NetCommand::JobStatus(target, event))
            .await
            .expect("Command should not be dropped");
    }
        
    // Share computer info to 
    pub async fn share_computer_info(&mut self, peer_id: PeerId) {
        self.sender
            .send(NetCommand::IncomingWorker(peer_id))
            .await
            .expect("Command should not have been dropped");
    }

    pub async fn start_providing(&mut self, file_name: String, path: PathBuf) {
        let (sender, receiver) = oneshot::channel();
        println!("Start providing file {file_name}");
        self.providing_files.insert(file_name.clone(), path);
        let cmd = NetCommand::StartProviding {
            file_name: file_name.clone(),
            sender,
        };
        self.sender
            .send(cmd)
            .await
            .expect("Command receiver not to be dropped");
        println!("Awaiting providing completion");
        receiver.await.expect("Sender should not be dropped");
        println!("File \"{file_name}\" is now available to download");
    }

    pub async fn get_providers(&mut self, file_name: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();

        println!("Calling get providers");
        self.sender
            .send(NetCommand::GetProviders {
                file_name: file_name.to_string(),
                sender,
            })
            .await
            .expect("Command receiver should not be dropped");

        println!("Awaiting provider result");
        receiver.await.expect("Sender should not be dropped")
    }

    pub async fn get_file_from_peers(
        &mut self,
        file_name: &str,
        destination: &PathBuf,
    ) -> Result<PathBuf, NetworkError> {
        let providers = self.get_providers(file_name).await;
        if providers.is_empty() {
            return Err(NetworkError::NoPeerProviderFound);
        }

        let requests = providers.into_iter().map(|p| {
            let mut client = self.clone();
            async move { client.request_file(p, file_name.to_owned()).await }.boxed()
        });

        let content = match futures::future::select_ok(requests).await {
            Ok(data) => data.0,
            Err(e) => {
                eprintln!("No peer found? {e:?}");
                return Err(NetworkError::NoPeerProviderFound);
            }
        };

        let file_path = destination.join(file_name);
        match async_std::fs::write(file_path.clone(), content).await {
            Ok(_) => Ok(file_path),
            Err(e) => Err(NetworkError::UnableToSave(e.to_string())),
        }
    }

    async fn request_file(
        &mut self,
        peer_id: PeerId,
        file_name: String,
    ) -> Result<Vec<u8>, Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::RequestFile {
                peer_id,
                file_name,
                sender,
            })
            .await
            .expect("Command should not be dropped");
        receiver.await.expect("Sender should not be dropped")
    }

    #[allow(dead_code)]
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
    
    // Used to collect computer information to distribute across network.
    machine: Machine,

    // Send Network event to subscribers.
    event_sender: Sender<NetEvent>,

    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_start_providing: HashMap<kad::QueryId, oneshot::Sender<()>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

impl NetworkService {
    // send command
    async fn handle_command(&mut self, cmd: NetCommand) {
        match cmd {
            NetCommand::Status(msg) => {
                let data = msg.as_bytes();
                let topic = IdentTopic::new(STATUS);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send status over network! {e:?}");
                }
            }
            NetCommand::RequestFile {
                peer_id,
                file_name,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer_id, FileRequest(file_name));
                self.pending_request_file.insert(request_id, sender);
            }
            NetCommand::RespondFile { file, channel } => {
                self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, FileResponse(file))
                    .expect("Connection to peer may still be open?");
            }
            NetCommand::IncomingWorker(peer_id) => {
                let spec = ComputerSpec::new(&mut self.machine);
                let data = bincode::serialize(&spec).unwrap();
                let topic = IdentTopic::new(SPEC);
                self.swarm.dial(peer_id);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send identity to swarm! {e:?}");
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
            NetCommand::StartProviding { file_name, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kad
                    .start_providing(file_name.into_bytes().into())
                    .expect("No store error.");

                self.pending_start_providing.insert(query_id, sender);
            }
            NetCommand::SubscribeTopic(topic) => {
                let ident_topic = IdentTopic::new(topic);
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&ident_topic)
                    .unwrap();
            }
            NetCommand::UnsubscribeTopic(topic) => {
                let ident_topic = IdentTopic::new(topic);
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .unsubscribe(&ident_topic)
                    .unwrap();
            }
            NetCommand::JobStatus(target, status) => {
                let data = bincode::serialize(&status).unwrap();
                // TODO: Find a way to send JobStatus to target peer machine?
                let topic = IdentTopic::new(JOB);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send job! {e:?}");
                }
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
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Kad(kad)) => {
                self.handle_kademila(kad).await
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::RequestResponse(rr)) => {
                self.handle_response(rr).await
            }
            // Once the swarm establish connection, we then send the peer_id we connected to.
            SwarmEvent::ConnectionEstablished { peer_id, ..  } => {
                self.event_sender.send(NetEvent::OnConnected(peer_id)).await.unwrap();
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                self.event_sender
                    .send(NetEvent::NodeDisconnected(peer_id))
                    .await
                    .unwrap();
            }
            _ => {
                println!("{event:?}")
            }
        }
    }

    async fn handle_response(
        &mut self,
        event: libp2p_request_response::Event<FileRequest, FileResponse>,
    ) {
        match event {
            libp2p_request_response::Event::Message { message, .. } => match message {
                libp2p_request_response::Message::Request {
                    request, channel, ..
                } => {
                    self.event_sender
                        .send(NetEvent::InboundRequest {
                            request: request.0,
                            channel,
                        })
                        .await
                        .expect("Event receiver should not be dropped!");
                }
                libp2p_request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    let _ = self
                        .pending_request_file
                        .remove(&request_id)
                        .expect("Request is still pending?")
                        .send(Ok(response.0));
                }
            },
            libp2p_request_response::Event::OutboundFailure {
                request_id, error, ..
            } => {
                let _ = self
                    .pending_request_file
                    .remove(&request_id)
                    .expect("Request is still pending")
                    .send(Err(Box::new(error)));
            }
            libp2p_request_response::Event::ResponseSent { .. } => {}
            _ => {}
        }
    }

    async fn handle_mdns(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, address) in peers {
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);

                    // add the discover node to kademlia list.
                    self.swarm
                        .behaviour_mut()
                        .kad
                        .add_address(&peer_id, address);
                }
            }
            mdns::Event::Expired(peers) => {
                for (peer_id, ..) in peers {
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            }
        };
    }

    // TODO: Figure out how I can use the match operator for TopicHash. I'd like to use the TopicHash static variable above.
    async fn handle_gossip(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message { message, .. } => match message.topic.as_str() {
                SPEC => {
                    let source = message.source.expect("Source cannot be empty!");
                    let specs =
                        bincode::deserialize(&message.data).expect("Fail to parse Computer Specs!");
                    if let Err(e) = self
                        .event_sender
                        .send(NetEvent::NodeDiscovered(source, specs))
                        .await
                    {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                STATUS => {
                    let source = message.source.expect("Source cannot be empty!");
                    let msg = String::from_utf8(message.data).unwrap();
                    if let Err(e) = self.event_sender.send(NetEvent::Status(source, msg)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                JOB => {
                    let peer_id = self.swarm.local_peer_id();
                    let job_event =
                        bincode::deserialize(&message.data).expect("Fail to parse Job data!");
                    if let Err(e) = self.event_sender.send(NetEvent::JobUpdate(peer_id.clone(), job_event)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                _ => {
                    let topic = message.topic.as_str();
                    let data = String::from_utf8(message.data).unwrap();
                    println!("Intercepted signal here? How to approach this? topic:{topic} | data:{data}");
                    // TODO: We may intercept signal for other purpose here, how can I do that?
                }
            },
            _ => {}
        }
    }

    // Handle kademila events (Used for file sharing)
    async fn handle_kademila(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed {
                id,
                result: kad::QueryResult::StartProviding(_),
                ..
            } => {
                let sender: oneshot::Sender<()> = self
                    .pending_start_providing
                    .remove(&id)
                    .expect("Completed query to be previously pending.");
                let _ = sender.send(());
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

    pub async fn run(mut self) {
        if let Err(e) = tokio::spawn(async move {
            loop {
                select! {
                    event = self.swarm.select_next_some() => self.handle_event(event).await,
                    Some(cmd) = self.command_receiver.recv() => self.handle_command(cmd).await,
                }
            }
        })
        .await
        {
            println!("fail to start background pool for network run! {e:?}");
        }
    }
}

impl AsRef<Receiver<NetCommand>> for NetworkService {
    fn as_ref(&self) -> &Receiver<NetCommand> {
        &self.command_receiver
    }
}
