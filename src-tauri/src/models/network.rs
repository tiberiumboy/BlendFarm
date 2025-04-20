use super::behaviour::{
    BlendFarmBehaviour, BlendFarmBehaviourEvent, FileRequest, FileResponse, FileService,
};
use super::computer_spec::ComputerSpec;
use super::job::JobEvent;
use super::message::{NetCommand, NetEvent, NetworkError};
use super::server_setting::ServerSetting;
use core::str;
use futures::{channel::oneshot, prelude::*};
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::kad::RecordKey;
use libp2p::swarm::SwarmEvent;
use libp2p::{kad, mdns, ping, swarm::Swarm, tcp, Multiaddr, PeerId, StreamProtocol, SwarmBuilder};
use libp2p_request_response::{ProtocolSupport, ResponseChannel};
use machine_info::Machine;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use std::u64;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::{io, select};

/*
Network Service - Receive, handle, and process network request.
*/

pub const STATUS: &str = "blendfarm/status";
pub const SPEC: &str = "blendfarm/spec";
pub const JOB: &str = "blendfarm/job";
pub const HEARTBEAT: &str = "blendfarm/heartbeat";
const TRANSFER: &str = "/file-transfer/1";

// the tuples return two objects
// Network Controller to interface network service
// Receiver<NetCommand> receive network events
pub async fn new() -> Result<(NetworkController, Receiver<NetEvent>), NetworkError> {
    // wonder if this is a good idea?
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

    // Begin listening on tcp and udp as server
    swarm
        .listen_on(tcp)
        .map_err(|e| NetworkError::UnableToListen(e.to_string()))?;

    swarm
        .listen_on(udp)
        .map_err(|e| NetworkError::UnableToListen(e.to_string()))?;

    // set the kad as server mode
    swarm.behaviour_mut().kad.set_mode(Some(kad::Mode::Server));

    // the command sender is used for outside method to send message commands to network queue
    let (sender, receiver) = mpsc::channel::<NetCommand>(32);

    // the event sender is used to handle incoming network message. E.g. RunJob
    let (event_sender, event_receiver) = mpsc::channel::<NetEvent>(32);

    let public_id = swarm.local_peer_id().clone();

    // start network service async
    let thread = tokio::spawn(async move {
        let mut network_service = NetworkService {
            swarm,
            receiver,
            sender: event_sender,
            // public_addr: None,
            machine: Machine::new(),
            // pending_dial: Default::default(),
            // TODO: job_service
            // pending_task: Default::default(),
        };
        network_service.run().await;
    });

    Ok((
        NetworkController {
            sender,
            file_service: FileService::new(),
            public_id,
            hostname: Machine::new().system_info().hostname,
            thread,
        },
        event_receiver,
    ))
}

// Network Controller interfaces network service.
pub struct NetworkController {
    // send net commands
    sender: mpsc::Sender<NetCommand>,

    // making it public until we can figure out how to use it correctly.
    pub public_id: PeerId,

    // must have this available somewhere.
    // Can we make this private?
    pub hostname: String,

    // Hmm? why does it need to be public?
    pub file_service: FileService,

    // network service background thread
    thread: JoinHandle<()>,
}

impl NetworkController {
    pub async fn subscribe_to_topic(&mut self, topic: String) {
        self.sender
            .send(NetCommand::SubscribeTopic(topic))
            .await
            .expect("sender should not be closed!");
    }

    pub async fn unsubscribe_from_topic(&mut self, topic: String) {
        self.sender
            .send(NetCommand::UnsubscribeTopic(topic))
            .await
            .expect("sender should not be closed!");
    }

    pub async fn send_node_status(&mut self, status: NodeEvent) {
        self.sender.send(NetCommand::NodeStatus(status)).await;
    }

    pub async fn send_status(&mut self, status: String) {
        println!("[Status]: {status}");
        self.sender
            .send(NetCommand::Status(status))
            .await
            .expect("Command should not been dropped");
    }

    // How do I get the peers info I want to communicate with?
    pub async fn send_job_message(&mut self, target: &str, event: JobEvent) {
        self.sender
            .send(NetCommand::JobStatus(target.to_string(), event))
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
        
        self.file_service
            .providing_files
            .insert(file_name.clone(), path);

        println!("Start providing file {:?}", &file_name);
        let cmd = NetCommand::StartProviding { file_name, sender };
        
        if let Err(e) = self.sender
            .send(cmd)
            .await
            {
                eprintln!("How did this happen? {e:?}");
            }

        // somehow receiver was dropped?
        if let Err(e) = receiver.await {
            eprintln!("Why did the receiver dropped? What happen?: {e:?}");
        }
    }

    pub async fn get_providers(&mut self, file_name: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::GetProviders {
                file_name: file_name.to_string(),
                sender,
            })
            .await
            .expect("Command receiver should not be dropped");
        
        // why was this dropped?
        match receiver.await {
            Ok(data) => data,
            Err(e) => {
                println!("Somehow this receiver was cancelled... Maybe there is no providers? {e:?}");
                HashSet::new()
            }
        }
    }

    // client request file from peers.
    // I feel like we should make this as fetching data from network? Some sort of stream?
    pub async fn get_file_from_peers(
        &mut self,
        file_name: &str,
        destination: &PathBuf,
    ) -> Result<PathBuf, NetworkError> {
        let providers = self.get_providers(&file_name).await;

        let content = match providers.iter().next() {
            Some(peer_id) => self.request_file(peer_id, file_name).await,
            None => return Err(NetworkError::NoPeerProviderFound),
        };

        match content {
            Ok(content) => {
                let file_path = destination.join(file_name);
                match async_std::fs::write(file_path.clone(), content).await {
                    Ok(_) => Ok(file_path),
                    Err(e) => Err(NetworkError::UnableToSave(e.to_string())),
                }
            }
            Err(e) => {
                // Received a "Timeout" error? What does that mean? Should I try to reconnect?
                eprintln!("No peer found? {e:?}");
                Err(NetworkError::Timeout)
            }
        }
    }

    pub async fn dial(
        &mut self,
        peer_id: PeerId,
        peer_addr: Multiaddr,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::Dial {
                peer_id,
                peer_addr,
                sender,
            })
            .await
            .expect("Command receiver should not be dropped");
        receiver.await.expect("Should not be closed?")
    }

    async fn request_file(
        &mut self,
        peer_id: &PeerId,
        file_name: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(NetCommand::RequestFile {
                peer_id: peer_id.clone(),
                file_name: file_name.into(),
                sender,
            })
            .await
            .expect("Command should not be dropped");
        receiver.await.expect("Should not be closed?") 
    }

    pub(crate) async fn respond_file(
        &mut self,
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    ) {
        let cmd = NetCommand::RespondFile { file, channel };
        if let Err(e) = self.sender.send(cmd).await {
            println!("Command should not be dropped: {e:?}");
        }
    }
}

// Network service module to handle invocation commands to send to network service,
// as well as handling network event from other peers
// Should use QUIC whenever possible!
pub struct NetworkService {
    // swarm behaviour - interface to the network
    swarm: Swarm<BlendFarmBehaviour>,

    // receive Network command
    receiver: Receiver<NetCommand>,

    // Send Network event to subscribers.
    sender: Sender<NetEvent>,

    // Used to collect computer basic hardware info to distribute
    machine: Machine,
    // current node address to reach/connect to - May not be needed?
    // public_addr: Option<Multiaddr>,

    // pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,

    // feels like we got a coupling nightmare here?
    // pending_task: HashMap<PeerId, oneshot::Sender<Result<Task, Box<dyn Error + Send>>>>,
}

// network service will be used to handle and receive network signal. It will also transmit network package over lan
impl NetworkService {
    pub fn get_host_name(&mut self) -> String {
        self.machine.system_info().hostname
    }

    // send command
    // is it possible to not use self?
    pub async fn handle_command(&mut self, cmd: NetCommand) {
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
                sender: snd,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer_id, FileRequest(file_name.into()));

                // so instead, we should just send a netevent?
                // so I think I was trying to send a sender channel here so that I could fetch the file content...
                // I received a request file command from UI - 
                // This instructs both things, a File Request was sent out to the network, and a notification to accept incoming transfer on this side.
                self.sender.send(NetEvent::PendingRequestFiled(request_id, Some(snd)));
            }
            NetCommand::RespondFile { file, channel } => {
                // somehow the send_response errored out? How come?
                // Seems like this function got timed out?
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    // TODO: find a way to get around cloning values.
                    .send_response(channel, FileResponse(file.clone()))
                {
                    // why am I'm getting error message here?
                    eprintln!("Error received on sending response!");
                }
            }
            NetCommand::IncomingWorker(..) => {
                let mut machine = Machine::new();
                let spec = ComputerSpec::new(&mut machine);
                let data = bincode::serialize(&spec).unwrap();
                let topic = IdentTopic::new(SPEC);

                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send identity to swarm! {e:?}");
                };
            }
            NetCommand::GetProviders {
                file_name, 
                sender: snd,
            } => {
                let key = RecordKey::new(&file_name.as_bytes());
                let query_id = self.swarm.behaviour_mut().kad.get_providers(key.into());
                self.sender.send(NetEvent::PendingGetProvider( query_id, snd)).await;
            }
            NetCommand::StartProviding { file_name, /*sender*/ .. } => {
                let provider_key = RecordKey::new(&file_name.as_bytes());
                let _query_id = self
                    .swarm
                    .behaviour_mut()
                    .kad
                    .start_providing(provider_key)
                    .expect("No store error.");

                //  todo, handle this somewhere else.
                // self.file_service
                //     .pending_start_providing
                //     .insert(query_id, sender);
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
                    .unsubscribe(&ident_topic);
            }
            // for the time being we'll use gossip.
            // TODO: For future impl. I would like to target peer by peer_id instead of host name.
            NetCommand::JobStatus(host_name, event) => {
                // convert data into json format.
                let data = bincode::serialize(&event).unwrap();

                // currently using a hack by making the target machine subscribe to their hostname.
                // the manager will send message to that specific hostname as target instead.
                // TODO: Read more about libp2p and how I can just connect to one machine and send that machine job status information.
                let topic = IdentTopic::new(host_name);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Error sending job status! {e:?}");
                }

                /*
                Let's break this down, we receive a worker with peer_id and peer_addr, both of which will be used to establish communication
                Once we establish a communication, that target peer will need to receive the pending task we have assigned for them.
                For now, we will try to dial the target peer, and append the task to our network service pool of pending task.
                */
                // self.pending_task.insert(peer_id);
            }
            NetCommand::NodeStatus(status) => {
                self.swarm.behaviour_mut().gossipsub.publish(topic, data)
            }
            NetCommand::Dial {
                peer_id,
                peer_addr,
                sender,
            } => {
                println!(
                    "Dialed: \nid:{:?}\naddr:{:?}\nsender:{:?}",
                    peer_id, peer_addr, sender
                );
                // Ok so where is this coming from?
                // if let hash_map::Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                //     behaviour
                //     .kad
                //     .add_address(&peer_id, peer_addr.clone());

                // match swarm.dial(peer_addr.with(Protocol::P2p(peer_id))) {
                //     Ok(()) => {
                //         e.insert(sender);
                //     }
                //     Err(e) => {
                //         let _ = sender.send(Err(Box::new(e)));
                //     }
                // }
            }
        }
    }

    // pub async fn handle_event(
    //     &mut self,
    //     sender: &mut Sender<NetEvent>,
    //     event: &SwarmEvent<BlendFarmBehaviourEvent>,
    // ) {
    //     match event {
    //         SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns)) => {
    //             self.handle_mdns(&mdns).await
    //         }
    //         SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossip)) => {
    //             Self::handle_gossip(sender, &gossip).await;
    //         }
    //         SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Kad(kad)) => {
    //             self.handle_kademila(&kad).await
    //         }
    //         SwarmEvent::Behaviour(BlendFarmBehaviourEvent::RequestResponse(rr)) => {
    //             Self::handle_response(sender, rr).await
    //         }
    //         // Once the swarm establish connection, we then send the peer_id we connected to.
    //         SwarmEvent::ConnectionEstablished { peer_id, .. } => {
    //             sender
    //                 .send(NetEvent::OnConnected(peer_id.clone()))
    //                 .await
    //                 .unwrap();
    //         }
    //         SwarmEvent::ConnectionClosed { peer_id, .. } => {
    //             sender
    //                 .send(NetEvent::NodeDisconnected(peer_id.clone()))
    //                 .await
    //                 .unwrap();
    //         }
    //         SwarmEvent::NewListenAddr { address, .. } => {
    //             // hmm.. I need to capture the address here?
    //             // how do I save the address?
    //             // this seems problematic?
    //             // if address.protocol_stack().any(|f| f.contains("tcp")) {
    //             //     self.public_addr = Some(address);
    //             // }
    //         }
    //         _ => {} //println!("[Network]: {event:?}");
    //     }
    // }

    async fn handle_response(
        &mut self,
        event: libp2p_request_response::Event<FileRequest, FileResponse>,
    ) {
        match event {
            libp2p_request_response::Event::Message { message, .. } => match message {
                libp2p_request_response::Message::Request {
                    request, channel, ..
                } => {
                    self.sender
                        .send(NetEvent::InboundRequest {
                            request: request.0,
                            channel: channel.into(),
                        })
                        .await
                        .expect("Event receiver should not be dropped!");
                }
                libp2p_request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    let value = response.0;
                    let event = NetEvent::ReceivedFileData(request_id, value);

                    self.sender
                        .send(event)
                        .await
                        .expect("Event receiver should not be dropped");
                }
            },
            libp2p_request_response::Event::OutboundFailure {
                request_id, error, ..
            } => {
                println!("Received outbound failure! {error:?}");
                if let Err(e) = self
                    .sender
                    .send(NetEvent::PendingRequestFiled(request_id, None))
                    .await
                {
                    eprintln!("Fail to send outbound failure! {e:?}");
                }
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
                        .add_address(&peer_id, address.clone());
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
                        .sender
                        .send(NetEvent::NodeDiscovered(source, specs))
                        .await
                    {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                STATUS => {
                    let source = message.source.expect("Source cannot be empty!");
                    // this looks like a bad idea... any how we could not use clone? stream?
                    let msg = String::from_utf8(message.data.clone()).unwrap();
                    if let Err(e) = self.sender.send(NetEvent::Status(source, msg)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                JOB => {
                    // let peer_id = self.swarm.local_peer_id();
                    let job_event = bincode::deserialize::<JobEvent>(&message.data)
                        .expect("Fail to parse Job data!");

                    // I don't think this function is called?
                    println!("Is this function used?");
                    if let Err(e) = self.sender.send(NetEvent::JobUpdate(job_event)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                // I think this needs to be changed.
                _ => {
                    // I received Mac.lan from message.topic?
                    let topic = message.topic.as_str();
                    if topic.eq(&self.machine.system_info().hostname) {
                        let job_event = bincode::deserialize::<JobEvent>(&message.data)
                            .expect("Fail to parse job data!");
                        
                        if let Err(e) = self.sender
                            .send(NetEvent::JobUpdate(job_event))
                            .await
                        {
                            eprintln!("Fail to send job update!\n{e:?}");
                        }

                    } else {
                        // let data = String::from_utf8(message.data).unwrap();
                        eprintln!("Intercepted unhandled signal here: {topic}");
                        // TODO: We may intercept signal for other purpose here, how can I do that?
                    }
                }
            },
            _ => {}
        }
    }

    // Handle kademila events (Used for file sharing)
    // thinking about transferring this to behaviour class?
    async fn handle_kademila(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed {
                // id,
                result: kad::QueryResult::StartProviding(_),
                ..
            } => {
                // let sender: oneshot::Sender<()> = self
                //     .file_service
                //     .pending_start_providing
                //     .remove(&id)
                //     .expect("Completed query to be previously pending.");
                // let _ = sender.send(());
            }
            kad::Event::OutboundQueryProgressed {
                // id,
                result:
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        // providers,
                        ..
                    })),
                ..
            } => {

                // if let Some(sender) = self.file_service.pending_get_providers.remove(&id) {
                //     sender
                //         .send(providers.clone())
                //         .expect("Receiver not to be dropped");
                //     self.kad.query_mut(&id).unwrap().finish();
                // }
            }
            kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                    )),
                ..
            } => {
                // what was suppose to happen here?
                println!(
                    r#"On OutboundQueryProgressed with result filter of 
                    FinishedWithNoAdditionalRecord: This should do something?"#
                );
            }
            
            // ignoring for now.
            kad::Event::InboundRequest { .. } => {}
            _ => {
                eprintln!("Unhandled Kademila event: {event:?}");
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<BlendFarmBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(behaviour) => match behaviour {
                BlendFarmBehaviourEvent::RequestResponse(event) => {
                    self.handle_response(event).await;
                }
                BlendFarmBehaviourEvent::Gossipsub(event) => {
                    self.handle_gossip(event).await;
                }
                BlendFarmBehaviourEvent::Mdns(event) => {
                    self.handle_mdns(event).await;
                }
                BlendFarmBehaviourEvent::Kad(event) => {
                    self.handle_kademila(event).await;
                }
                BlendFarmBehaviourEvent::Ping(event) => {
                    eprintln!("{event:?}");
                }
            },
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if let Err(e) = self.sender.send(NetEvent::OnConnected(peer_id)).await {
                    eprintln!("Fail to send event on connection established! {e:?}");
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                if let Err(e) = self.sender.send(NetEvent::NodeDisconnected(peer_id)).await {
                    eprintln!("Fail to send event on connection closed! {e:?}");
                }
            }

            // hmm?
            // SwarmEvent::IncomingConnection {
            //     connection_id,
            //     local_addr,
            //     send_back_addr,
            // } => {
            //     todo!()
            // }

            // hmm?
            // SwarmEvent::IncomingConnectionError { .. } => {}
            // SwarmEvent::OutgoingConnectionError { .. } => {}
            // SwarmEvent::NewListenAddr { .. } => {}
            // SwarmEvent::ExpiredListenAddr { .. } => {}

            // SwarmEvent::ListenerClosed { .. } => todo!(),
            // SwarmEvent::ListenerError { listener_id, error } => todo!(),

            // SwarmEvent::Dialing { .. } => todo!(),
            // SwarmEvent::NewExternalAddrCandidate { address } => todo!(),
            // SwarmEvent::ExternalAddrConfirmed { address } => todo!(),
            // hmm?
            // SwarmEvent::ExternalAddrExpired { address } => {}
            SwarmEvent::NewExternalAddrOfPeer { peer_id, .. } => {
                if let Err(e) = self.sender.send(NetEvent::OnConnected(peer_id)).await {
                    eprintln!("{e:?}");
                }
            }
            // we'll do nothing for this for now.
            _ => {}
        };
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                Some(msg) = self.receiver.recv() => self.handle_command(msg).await,
                Some(event) = self.swarm.next() => self.handle_event(event).await,
            }
        }
    }
}

// impl AsRef<Receiver<NetCommand>> for NetworkService {
//     fn as_ref(&self) -> &Receiver<NetCommand> {
//         &self.command_receiver
//     }
// }
