use super::behaviour::{BlendFarmBehaviour, FileRequest, FileResponse};
use super::computer_spec::ComputerSpec;
use super::job::{Job, JobError};
use super::message::{NetCommand, NetEvent, NetworkError};
use super::server_setting::ServerSetting;
use crate::models::behaviour::BlendFarmBehaviourEvent;
use core::str;
use std::path::PathBuf;
use futures::channel::oneshot;
use libp2p::futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic},
    kad, mdns,
    swarm::{Swarm, SwarmEvent},
    tcp, Multiaddr, SwarmBuilder,
};
use libp2p::{PeerId, StreamProtocol};
use libp2p_request_response::{OutboundRequestId, ProtocolSupport, ResponseChannel};
use machine_info::Machine;
use std::collections::{HashMap, HashSet};
use std::error::Error;
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
        },
        event_receiver,
    ))
}

#[derive(Clone)]
pub struct NetworkController {
    sender: mpsc::Sender<NetCommand>,
    pub settings: ServerSetting,
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

    pub async fn request_job(&mut self, event: Option<JobError>) {
        let cmd = match event {
            Some(err) => NetCommand::JobFailure(err),
            None => NetCommand::RequestJob, 
        };
        self.sender.send(cmd).await.unwrap();
    }

    pub async fn send_status(&mut self, status: String) {
        self.sender
            .send(NetCommand::Status(status))
            .await
            .expect("Command should not have been dropped");
    }

    // may not be in use?
    pub async fn share_computer_info(&mut self) {
        self.sender
            .send(NetCommand::SendIdentity)
            .await
            .expect("Command should not have been dropped");
    }

    pub async fn start_providing(&mut self, file_name: String, path: PathBuf) {
        let (sender, receiver) = oneshot::channel();
        let cmd = NetCommand::StartProviding { file_name, path, sender };
        self.sender
            .send(cmd)
            .await
            .expect("Command receiver not to be dropped");
        receiver.await.expect("Sender should not be dropped");
    }

    pub async fn get_providers(&mut self, file_name: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::GetProviders { file_name: file_name.to_string(), sender })
            .await
            .expect("Command receiver should not be dropped");
        receiver.await.expect("Sender should not be dropped")
    }

    pub async fn send_network_job(&mut self, job: Job) {
        self.sender
            .send(NetCommand::StartJob(job))
            .await
            .expect("Command should not have been dropped!");
    }

    pub(crate) async fn request_file(
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
    machine: Machine,
    // send network events
    event_sender: Sender<NetEvent>,
    files_providing: HashMap<String, PathBuf>,
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_start_providing: HashMap<kad::QueryId, oneshot::Sender<()>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

impl NetworkService {
    // send command
    async fn handle_command(&mut self, cmd: NetCommand) {
        match cmd {
            // Begin the job
            // The idea here is that we received a new job from the host -
            // we would need to upload blender to kad service and make it public available for DHT to access for other nodes to obtain
            // then we send out notification to all of the node to start the job
            NetCommand::StartJob(job) => {
                // receives a job request. can do fancy behaviour like split up the job into different frames?
                // TODO: For now, send the job request.
                let data = bincode::serialize(&job).unwrap();
                let topic = IdentTopic::new(JOB);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send job! {e:?}");
                }
            }
            // Send message to all other peer to stop the target job ID and remove from kad provider
            NetCommand::EndJob { .. } => todo!(),
            // send status update
            NetCommand::Status(msg) => {
                let data = msg.as_bytes();
                let topic = IdentTopic::new(STATUS);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send status over network! {e:?}");
                }
            }

            // TODO: For Future impl. See how we can transfer the file using kad's behaviour (DHT)
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
            NetCommand::SendIdentity => {
                let spec = ComputerSpec::new(&mut self.machine);
                let data = bincode::serialize(&spec).unwrap();
                let topic = IdentTopic::new(SPEC);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to publish message to swarm! {e:?}");
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
            NetCommand::StartProviding { file_name, path, sender } => {
                self.files_providing.insert(file_name.clone(), path );
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
            NetCommand::RequestJob => {
                // hmm I assume a node is asking the host for job?
                // will have to come back for this one and think.
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
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Kad(kad)) => {
                self.handle_kademila(kad).await
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::RequestResponse(rr)) => {
                self.handle_response(rr).await
            }
            SwarmEvent::ConnectionEstablished { .. } => {
                self.event_sender.send(NetEvent::OnConnected).await.unwrap();
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
                    .expect("Request to is still pending")
                    .send(Err(Box::new(error)));
            }
            libp2p_request_response::Event::ResponseSent { .. } => {}
            _ => {}
        }
    }

    // TODO: Haven't found a place for this yet, but still thinking about how to handle node disconnection?
    // async fn remove_peer(&mut self, peer_id: PeerId) {
    //     // send a message back notifying a node was disconnnected
    //     let event = NetEvent::NodeDisconnected(peer_id);
    //     if let Err(e) = self.event_sender.send(event).await {
    //         println!("Error sending node disconnected signal to UI: {e:?}");
    //     }
    // }

    async fn handle_mdns(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, ..) in peers {
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
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
                    let job: Job =
                        bincode::deserialize(&message.data).expect("Fail to parse Job data!");
                    if let Err(e) = self.event_sender.send(NetEvent::Render(job)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                _ => eprintln!(
                    "Received unknown topic hash! Potential malicious foreign command received?"
                ),
            },
            _ => {}
        }
    }

    async fn handle_kademila(&mut self, event: kad::Event) {
        println!("Receive kademila service request");
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
