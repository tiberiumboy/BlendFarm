use super::behaviour::{
    BlendFarmBehaviour, BlendFarmBehaviourEvent, FileRequest, FileResponse,
};
use super::computer_spec::ComputerSpec;
use super::job::JobEvent;
use super::message::{Command, Event, NetworkError, Target};
use core::str;
use std::str::FromStr;
use futures::{channel::{mpsc::{self, Receiver, Sender}, oneshot}, prelude::*};
use libp2p::gossipsub::{self, IdentTopic, Message};
use libp2p::identity;
use libp2p::kad::{QueryId, RecordKey};
use libp2p::swarm::SwarmEvent;
use libp2p::{kad, mdns, ping, swarm::Swarm, tcp, Multiaddr, PeerId, StreamProtocol, SwarmBuilder};
use libp2p_request_response::{OutboundRequestId, ProtocolSupport, ResponseChannel};
use machine_info::Machine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{PathBuf, Path};
use std::time::Duration;
use std::u64;
use tokio::{io, select};

use futures::StreamExt;

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
pub async fn new(secret_key_seed: Option<u8>) -> Result<(NetworkController, Receiver<Event>, NetworkService), NetworkError> {
    // wonder if this is a good idea?
    let duration = Duration::from_secs(u64::MAX);
    let id_keys = match secret_key_seed { 
        Some(seed) => {
            let mut bytes = [0u8; 32];
            bytes[0] = seed;
            identity::Keypair::ed25519_from_bytes(bytes).unwrap()
        }
        None => identity::Keypair::generate_ed25519()
    };
    let tcp_config: tcp::Config = tcp::Config::default();

    let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
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
    let (sender, receiver) = mpsc::channel::<Command>(32);

    // the event sender is used to handle incoming network message. E.g. RunJob
    let (event_sender, event_receiver) = mpsc::channel::<Event>(32);

    let public_id = swarm.local_peer_id().clone();

    let controller = NetworkController {
        sender,
        public_id,
        hostname: Machine::new().system_info().hostname,
    };

    let service = NetworkService::new(
        swarm,
        receiver,
        event_sender, // Here is where network service communicates out.
    );

    Ok((
        controller,
        event_receiver,
        service
    ))
}

// Network Controller interfaces network service.
#[derive(Clone)]
pub struct NetworkController {
    sender: mpsc::Sender<Command>, // send net commands
    pub public_id: PeerId,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusEvent {
    Online,
    Busy,
    Offline,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerIdString {
    inner: String
}

// Must be serializable to send data across network
#[derive(Debug, Serialize, Deserialize)] // Clone,
pub enum NodeEvent {
    Discovered(PeerIdString, ComputerSpec),
    Disconnected(PeerIdString),
    Status(StatusEvent)
}

impl PeerIdString {
    pub fn new(peer: &PeerId) -> Self {
        Self {
            inner: peer.to_base58()
        }
    }

    pub fn to_peer_id(self) -> PeerId {
        PeerId::from_str(&self.inner).expect("Should not fail?")
    }
}

impl NetworkController {
    pub async fn subscribe_to_topic(&mut self, topic: String) {
        self.sender
            .send(Command::SubscribeTopic(topic))
            .await
            .expect("sender should not be closed!");
    }

    pub async fn unsubscribe_from_topic(&mut self, topic: String) {
        self.sender
            .send(Command::UnsubscribeTopic(topic))
            .await
            .expect("sender should not be closed!");
    }

    // 
    pub async fn send_node_status(&mut self, status: NodeEvent) {
        if let Err(e) = self.sender.send(Command::NodeStatus(status)).await {
            eprintln!("Failed to send node status to network service: {e:?}");
        }
    }

    pub async fn send_status(&mut self, status: String) {
        println!("[Status]: {status}");
        self.sender
            .send(Command::Status(status))
            .await
            .expect("Command should not been dropped");
    }

    // How do I get the peers info I want to communicate with?
    // Try to use DHT as chat post instead - Delete message if no longer providing over the network
    pub async fn send_job_message(&mut self, target: Target, event: JobEvent) {
        self.sender
            .send(Command::JobStatus(target, event))
            .await
            .expect("Command should not be dropped");
    }

    // Share computer info to
    pub async fn share_computer_info(&mut self, peer_id: PeerId) {
        self.sender
            .send(Command::IncomingWorker(peer_id))
            .await
            .expect("Command should not have been dropped");
    }

    /// file_name are broadcasted with the extensions included, but not the directory it's located in. E.g. "test.blend"
    pub async fn start_providing(&mut self, path: PathBuf) {
        
        // what was the whole idea of using the receiver?
        let cmd = Command::StartProviding(path);
        
        if let Err(e) = self.sender
        .send(cmd)
        .await
            {
                eprintln!("How did this happen? {e:?}");
            }

        // somehow receiver was dropped?
        // what are we receiving/awaiting for? 
        // if let Err(e) = receiver.await {
        //     eprintln!("Why did the receiver dropped? What happen?: {e:?}");
        // }
    }

    pub async fn get_providers(&mut self, file_name: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::GetProviders {
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
    pub async fn get_file_from_peers<T: AsRef<Path>>(
        &mut self,
        file_name: &str,
        destination: T,
    ) -> Result<PathBuf, NetworkError> {
        let providers = self.get_providers(&file_name).await;

        let content = match providers.iter().next() {
            Some(peer_id) => self.request_file(peer_id, file_name).await,
            None => return Err(NetworkError::NoPeerProviderFound),
        };

        match content {
            Ok(content) => {
                let file_path = destination.as_ref().join(file_name);
                // TODO: See if we can re-write this better? Should be able to map this?
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

    async fn request_file(
        &mut self,
        peer_id: &PeerId,
        file_name: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestFile {
                peer_id: peer_id.clone(),
                file_name: file_name.into(),
                sender,
            })
            .await
            .expect("Command should not be dropped");
        receiver.await.expect("Should not be closed?") 
    }

    // TODO: Come back to this one and see how this one gets invoked.
    pub(crate) async fn respond_file(
        &mut self,
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    ) {
        let cmd = Command::RespondFile { file, channel };
        if let Err(e) = self.sender.send(cmd).await {
            println!("Command should not be dropped: {e:?}");
        }
    }
}

// Network service module to handle invocation commands to send to network service,
// as well as handling network event from other peers
pub struct NetworkService {
    // swarm behaviour - interface to the network
    swarm: Swarm<BlendFarmBehaviour>,

    // receive Network command
    receiver: Receiver<Command>,

    // Send Network event to subscribers.
    sender: Sender<Event>,

    // Used to collect computer basic hardware info to distribute
    machine: Machine,

    providing_files: HashMap<QueryId, PathBuf>,
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    // hmm?
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

// network service will be used to handle and receive network signal. It will also transmit network package over lan
impl NetworkService {
    pub fn new(swarm: Swarm<BlendFarmBehaviour>, receiver: Receiver<Command>, sender: Sender<Event>) -> NetworkService {
        Self {
            swarm,
            receiver,
            sender,
            machine: Machine::new(),
            providing_files: Default::default(),
            pending_get_providers: Default::default(),
            pending_request_file: Default::default(),
        }
    }

    pub fn get_host_name(&mut self) -> String {
        self.machine.system_info().hostname
    }

    // send command
    // is it possible to not use self?
    pub async fn process_command(&mut self, cmd: Command) {
        match cmd {
            Command::Status(msg) => {
                let data = msg.as_bytes();
                let topic = IdentTopic::new(STATUS);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send status over network! {e:?}");
                }
            }
            Command::RequestFile {
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
                if let Err(e) = self.sender.send(Event::PendingRequestFiled(request_id, Some(snd))).await {
                    eprintln!("Failed to send file contents: {e:?}");
                }
            }
            Command::RespondFile { file, channel } => {
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
                    eprintln!("Error received on sending response! {e:?}");
                }
            }
            Command::IncomingWorker(..) => {
                let mut machine = Machine::new();
                let spec = ComputerSpec::new(&mut machine);
                let data = bincode::serialize(&spec).unwrap();
                let topic = IdentTopic::new(SPEC);

                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to send identity to swarm! {e:?}");
                };
            }
            Command::GetProviders {
                file_name, 
                sender: snd,
            } => {
                let key = RecordKey::new(&file_name.as_bytes());
                let query_id = self.swarm.behaviour_mut().kad.get_providers(key.into());
                if let Err(e) = self.sender.send(Event::PendingGetProvider( query_id, snd)).await {
                    eprintln!("Fail to send provider data. {e:?}");
                }
            }
            Command::StartProviding (file_path) => {
                let file_name = file_path.file_name().expect("Must be a valid file");

                let provider_key = RecordKey::new(&file_name.as_encoded_bytes());
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kad
                    .start_providing(provider_key)
                    .expect("No store error."); 

                self.providing_files.insert(query_id, file_path);
            }
            Command::SubscribeTopic(topic) => {
                let ident_topic = IdentTopic::new(topic);
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&ident_topic)
                    .unwrap();
            }
            Command::UnsubscribeTopic(topic) => {
                let ident_topic = IdentTopic::new(topic);
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .unsubscribe(&ident_topic);
            }
            Command::JobStatus(host_name, event) => {
                // convert data into json format.
                let data = bincode::serialize(&event).unwrap();

                // currently using a hack by making the target machine subscribe to their hostname.
                // the manager will send message to that specific hostname as target instead.
                // TODO: Read more about libp2p and how I can just connect to one machine and send that machine job status information.
                let name = match host_name { 
                    Some(name) => name,
                    None => JOB.to_owned(),
                };
                
                let topic = IdentTopic::new(name);
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
            // TODO: need to figure out how this is called. 
            Command::NodeStatus(status) => {
                // we want to send this info across broadcast network. We do not care who is listening the network. Only the fact that we want our hosts to keep notify for availability.
                // where did we get 
                let topic = IdentTopic::new(STATUS);
                let data = bincode::serialize(&status).unwrap();
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to publish gossip message: {e:?}");
                }
            }
        }
    }

    async fn process_response_event(
        &mut self,
        event: libp2p_request_response::Event<FileRequest, FileResponse>,
    ) {
        match event {
            libp2p_request_response::Event::Message { message, .. } => match message {
                libp2p_request_response::Message::Request {
                    request, channel, ..
                } => {
                    self.sender
                        .send(Event::InboundRequest {
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
                    let event = Event::ReceivedFileData(request_id, value);

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
                    .send(Event::PendingRequestFiled(request_id, None))
                    .await
                {
                    eprintln!("Fail to send outbound failure! {e:?}");
                }
            }
            libp2p_request_response::Event::ResponseSent { .. } => {}
            _ => {}
        }
    }

    async fn process_mdns_event(&mut self, event: mdns::Event) {
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

    async fn handle_spec(&mut self, source: PeerId, message: Message ) {
        // deserialize message into structure data. We expect this. Run unit test for null/invalid datastruct/malicious exploits.
        if let Ok(specs) = bincode::deserialize(&message.data) {
            // send a net event notification 
            if let Err(e) = self
                .sender
                .send(Event::NodeDiscovered(source, specs))
                .await
            {
                eprintln!("Something failed? {e:?}");
            }
        }
    }

    async fn handle_status(&mut self, source : PeerId, message: Message) {
        // this looks like a bad idea... any how we could not use clone? stream?
        let msg = String::from_utf8(message.data.clone()).unwrap();
        if let Err(e) = self.sender.send(Event::Status(source, msg)).await {
            eprintln!("Something failed? {e:?}");
        }
    }

    async fn handle_job(&mut self, message: Message) {
        // let peer_id = self.swarm.local_peer_id();
        let job_event = bincode::deserialize::<JobEvent>(&message.data)
            .expect("Fail to parse Job data!");

        // I don't think this function is called?
        println!("Is this function used?");
        if let Err(e) = self.sender.send(Event::JobUpdate(job_event)).await {
            eprintln!("Something failed? {e:?}");
        }
    }

    // TODO: Figure out how I can use the match operator for TopicHash. I'd like to use the TopicHash static variable above.
    async fn process_gossip_event(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message { propagation_source, message, .. } => match message.topic.as_str() {
                // when we received a SPEC topic.
                SPEC => {                    
                    self.handle_spec(propagation_source, message).await;
                }
                STATUS => {
                    self.handle_status(propagation_source, message).await;
                }
                JOB => {
                    self.handle_job(message).await;
                }
                // I think this needs to be changed.
                _ => {
                    // I received Mac.lan from message.topic?
                    let topic = message.topic.as_str();
                    if topic.eq(&self.machine.system_info().hostname) {
                        let job_event = bincode::deserialize::<JobEvent>(&message.data)
                            .expect("Fail to parse job data!");
                        
                        if let Err(e) = self.sender
                            .send(Event::JobUpdate(job_event))
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
    async fn process_kademlia_event(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed {
                // id,
                result: kad::QueryResult::StartProviding(providers),
                ..
            } => {
                println!("Received OutboundQueryProgressed: {providers:?}");
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
                        providers,
                        ..
                    })),
                ..
            } => {
                
                // So, here's where we finally receive the invocation?
                if let Some(sender) = self.pending_get_providers.remove(&id) {
                //     sender
                //         .send(providers.clone())
                //         .expect("Receiver not to be dropped");
                //     self.kad.query_mut(&id).unwrap().finish();
                }
            }
            kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { closest_peers },
                    )),
                ..
            } => {
                // This piece of code means that there's nobody advertising this on the network?
                // what was suppose to happen here?
                // TODO: I am once again stopped here. This message appeared from the CLI side. Not the host.
                
                let outbound_request_id = ???
                let event = Event::PendingRequestFiled(outbound_request_id, None);
                self.sender.send(event).await;
            }

            
            // suppressed
            kad::Event::OutboundQueryProgressed { result: kad::QueryResult::Bootstrap(..), .. } => {}
            // suppressed
            kad::Event::InboundRequest { .. } => {}
            // suppressed
            kad::Event::RoutingUpdated { .. } => {}
            _ => {
                // oh mah gawd. What am I'm suppose to do here?
                eprintln!("Unhandled Kademila event: {event:?}");
            }
        }
    }

    async fn process_swarm_event(&mut self, event: SwarmEvent<BlendFarmBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(behaviour) => match behaviour {
                BlendFarmBehaviourEvent::RequestResponse(event) => {
                    self.process_response_event(event).await;
                }
                BlendFarmBehaviourEvent::Gossipsub(event) => {
                    self.process_gossip_event(event).await;
                }
                BlendFarmBehaviourEvent::Mdns(event) => {
                    self.process_mdns_event(event).await;
                }
                BlendFarmBehaviourEvent::Kad(event) => {
                    self.process_kademlia_event(event).await;
                }
            },
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if let Err(e) = self.sender.send(Event::OnConnected(peer_id)).await {
                    eprintln!("Fail to send event on connection established! {e:?}");
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                if let Err(e) = self.sender.send(Event::NodeDisconnected(peer_id)).await {
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
            // SwarmEvent::ListenerClosed { .. } => todo!(),
            // SwarmEvent::ListenerError { listener_id, error } => todo!(),
            // SwarmEvent::Dialing { .. } => todo!(),

            SwarmEvent::NewExternalAddrOfPeer { peer_id, .. } => {
                if let Err(e) = self.sender.send(Event::OnConnected(peer_id)).await {
                    eprintln!("{e:?}");
                }
            }
            // we'll do nothing for this for now.
            // see what we're skipping?
            _ => { println!("[Network]: {event:?}"); }
        };
    }

    // pub async fn handle_event(
    //     &mut self,
    //     sender: &mut Sender<NetEvent>,
    //     event: &SwarmEvent<BlendFarmBehaviourEvent>,
    // ) {
    //     match event {
    //         SwarmEvent::NewListenAddr { address, .. } => {
    //             // hmm.. I need to capture the address here?
    //             // how do I save the address?
    //             // this seems problematic?
    //             // if address.protocol_stack().any(|f| f.contains("tcp")) {
    //             //     self.public_addr = Some(address);
    //             // }
    //         }
    //     }
    // }

    pub async fn run(&mut self) {
        loop {
            select! {
                msg = self.receiver.select_next_some() => self.process_command(msg).await,
                event = self.swarm.select_next_some() => self.process_swarm_event(event).await,
            }
        }
    }
}