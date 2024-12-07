use super::behaviour::{BlendFarmBehaviour, FileRequest, FileResponse};
use super::computer_spec::ComputerSpec;
use super::job::Job;
use super::message::{NetCommand, NetEvent, NetworkError};
use crate::models::behaviour::BlendFarmBehaviourEvent;
use core::str;
use futures::channel::oneshot;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::SubscriptionError;
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

const STATUS: &str = "blendfarm/status";
const SPEC: &str = "blendfarm/spec";
const JOB: &str = "blendfarm/job";
const HEARTBEAT: &str = "blendfarm/heartbeat";
const TRANSFER: &str = "/file-transfer/1";

fn subscribe(
    swarm: &mut Swarm<BlendFarmBehaviour>,
    topic: &str,
) -> Result<bool, SubscriptionError> {
    let ident_topic = IdentTopic::new(topic);
    swarm.behaviour_mut().gossipsub.subscribe(&ident_topic)
}

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
                // .heartbeat_interval(Duration::from_secs(10))
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

    subscribe(&mut swarm, STATUS).unwrap();
    subscribe(&mut swarm, SPEC).unwrap();
    subscribe(&mut swarm, JOB).unwrap();
    subscribe(&mut swarm, HEARTBEAT).unwrap();

    let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
        .parse()
        .map_err(|e| NetworkError::BadInput)?;
    let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0"
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
        },
        event_receiver,
    ))
}

#[derive(Clone)]
pub struct NetworkController {
    sender: mpsc::Sender<NetCommand>,
}

impl NetworkController {
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
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_start_providing: HashMap<kad::QueryId, oneshot::Sender<()>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

impl NetworkService {
    /*

        TODO: Figure out what I was suppose to do with this file?

    //         NetEvent::Render(job) => {
        //             // Here we'll check the job -
        //             // TODO: It would be nice to check and see if there's any jobs currently running, otherwise put it in a poll?
        //             let project_file = job.project_file;
        //             let version: &Version = project_file.as_ref();
        //             let blender = self
        //                 .manager
        //                 .fetch_blender(version)
        //                 .expect("Should have blender installed?");
        //             let file_path: &Path = project_file.as_ref();
        //             let args = Args::new(file_path, job.output, job.mode);
        //             let rx = blender.render(args);
        // for this particular loop, let's extract this out to simplier function.
        // loop {
        //         if let Ok(msg) = rx.recv() {
        //             let msg = match msg {
        //                 Status::Idle => "Idle".to_owned(),
        //                 Status::Running { status } => status,
        //                 Status::Log { status } => status,
        //                 Status::Warning { message } => message,
        //                 Status::Error(err) => format!("{err:?}").to_owned(),
        //                 Status::Completed { result } => {
        //                     // we'll send the message back?
        //                     // net_service
        //                     // here we will state that the render is complete, and send a message to network service
        //                     // TODO: Find a better way to not use the `.clone()` method.
        //                     let msg = Command::FrameCompleted(
        //                         result.clone(),
        //                         job.current_frame,
        //                     );
        //                     let _ = net_service.send(msg).await;
        //                     let path_str = &result.to_string_lossy();
        //                     format!(
        //                         "Finished job frame {} at {path_str}",
        //                         job.current_frame
        //                     )
        //                     .to_owned()
        //                     // here we'll send the job back to the peer who requested us the job initially.
        //                     // net_service.swarm.behaviour_mut().gossipsub.publish( peer_id, )
        //                 }
        //             };
        //             println!("[Status] {msg}");
        //         }
        //             // }
        //         }
        // }

    */

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
                // how do I sent the specs to only the computer I want to communicate to or connect to?
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
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
            NetCommand::StartProviding { file_name, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kad
                    .start_providing(file_name.into_bytes().into())
                    .expect("No store error.");
                self.pending_start_providing.insert(query_id, sender);
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

            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if let Err(e) = self
                    .event_sender
                    .send(NetEvent::NodeDiscovered(peer_id))
                    .await
                {
                    eprintln!("Fail to send node discovery from establishment {e:?}");
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                self.remove_peer(peer_id).await;
            }

            // omitting message.
            SwarmEvent::NewListenAddr { address, .. } => println!("Now listening on {address}"),
            SwarmEvent::IncomingConnection { local_addr, .. } => {
                println!("Receiving Incoming Connection... {local_addr}")
            }
            SwarmEvent::Dialing { peer_id, .. } => match peer_id {
                Some(peer_id) => println!("Dialing {peer_id}..."),
                None => println!("Dialing no peer_id?"),
            },
            // I am getting this strange InappropriateHandshakeMessage after Node is discovered?
            SwarmEvent::OutgoingConnectionError { error, .. } => {
                // More likely The address is in used, but I'm not sure why or how that's possible?
                println!("Received outgoing connection error: {error:?}");
            }
            SwarmEvent::IncomingConnectionError { error, .. } => {
                println!("Received incoming connection error: {error:?}");
            }
            _ => {
                println!("Unhandle swarm behaviour event: {event:?}")
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

    fn add_peer(&mut self, peer_id: &PeerId) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .add_explicit_peer(&peer_id);
    }

    async fn remove_peer(&mut self, peer_id: PeerId) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .remove_explicit_peer(&peer_id);

        // send a message back notifying a node was disconnnected
        let event = NetEvent::NodeDisconnected(peer_id);
        if let Err(e) = self.event_sender.send(event).await {
            println!("Error sending node disconnected signal to UI: {e:?}");
        }
    }

    async fn handle_mdns(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, ..) in list {
                    self.add_peer(&peer_id);
                    // send a message back confirming a node is discoverable (Use this to populate UI element on the front end facing app)
                    let event = NetEvent::NodeDiscovered(peer_id);
                    if let Err(e) = self.event_sender.send(event).await {
                        println!("Error sending node discovered signal to UI: {e:?}");
                    }
                }
            }
            mdns::Event::Expired(list) => {
                for (peer_id, ..) in list {
                    self.remove_peer(peer_id).await;
                }
            } // _ => {}
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
                        .send(NetEvent::Identity(source, specs))
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
                    let source = message.source.expect("Source cannot be empty!");
                    let job: Job =
                        bincode::deserialize(&message.data).expect("Fail to parse Job data!");
                    if let Err(e) = self.event_sender.send(NetEvent::Render(source, job)).await {
                        eprintln!("SOmething failed? {e:?}");
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
