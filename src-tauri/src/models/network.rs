use super::behaviour::{BlendFarmBehaviour, BlendFarmBehaviourEvent, FileRequest, FileResponse};
use super::job::JobEvent;
use super::message::{Command, Event, FileCommand, KeywordSearch, NetworkError};
use blender::models::event::BlenderEvent;
use core::str;
use futures::StreamExt;
use futures::{
    channel::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    prelude::*,
};
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::kad::RecordKey;
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{Multiaddr, PeerId, StreamProtocol, SwarmBuilder, kad, mdns, noise, tcp, yamux};
use libp2p_request_response::{OutboundRequestId, ProtocolSupport, ResponseChannel};
use machine_info::Machine;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::u64;
use tokio::{io, select};

/*
Network Service - Receive, handle, and process network request.
*/

// what is status? If it's not job status nor node status?
const STATUS: &str = "/blendfarm/status";
const JOB: &str = "/blendfarm/job";
const NODE: &str = "/blendfarm/node";
// why does the transfer have number at the trail end? look more into this?
const TRANSFER: &str = "/file-transfer/1";

pub enum ProviderRule {
    // Use "file name.ext", Extracted from PathBuf.
    Default(PathBuf),
    // Custom keyword search for specific PathBuf.
    Custom(KeywordSearch, PathBuf),
}

// the tuples return two objects
// Network Controller to interface network service
// Receiver<NetCommand> receive network events
pub async fn new() -> Result<(NetworkController, Receiver<Event>, NetworkService), NetworkError> {
    // wonder why we have a connection timeout of 60 seconds? Why not uint::MAX?
    let duration = Duration::from_secs(60);
    // is there a reason for the secret key seed?
    // let id_keys = match secret_key_seed {
    //     Some(seed) => {
    //         let mut bytes = [0u8; 32];
    //         bytes[0] = seed;
    //         identity::Keypair::ed25519_from_bytes(bytes).unwrap()
    //     }
    //     None => identity::Keypair::generate_ed25519(),
    // };

    // let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("Should be able to build with tcp configuration?")
        .with_quic()
        .with_behaviour(|key| {
            // seems like we need to content-address message. We'll use the hash of the message as the ID.
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

            // p2p communication
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .expect("Fail to create gossipsub behaviour");

            // network discovery usage
            // TODO: replace expect with error handling
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                    .expect("Fail to create mdns behaviour!");

            // Used to provide file provision list
            let kad = kad::Behaviour::new(
                key.public().to_peer_id(),
                kad::store::MemoryStore::new(key.public().to_peer_id()),
            );

            let rr_config = libp2p_request_response::Config::default();
            // Learn more about this and see if we need the transfer keyword of some sort?
            let protocol = [(StreamProtocol::new(TRANSFER), ProtocolSupport::Full)];
            let request_response = libp2p_request_response::Behaviour::new(protocol, rr_config);

            Ok(BlendFarmBehaviour {
                request_response,
                gossipsub,
                mdns,
                kad,
            })
        })
        // TODO remove/handle expect()
        .expect("Expect to build behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
        .build();

    // Listen on all interfaces and whatever port OS assigns
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

    // all network interference must subscribe to these topics!
    let job_topic = gossipsub::IdentTopic::new(JOB);
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&job_topic) {
        eprintln!("Fail to subscribe job topic! {e:?}");
    }

    let node_topic = gossipsub::IdentTopic::new(NODE);
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&node_topic) {
        eprintln!("Fail to subscribe node topic! {e:?}");
    }

    let service = NetworkService::new(
        swarm,
        receiver,
        event_sender, // Here is where network service communicates out.
    );

    Ok((controller, event_receiver, service))
}

// Network Controller interfaces network service.
#[derive(Clone)]
pub struct NetworkController {
    sender: mpsc::Sender<Command>, // send net commands
    pub public_id: PeerId,
    pub hostname: String,
}

// what is StatusEvent responsibility?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusEvent {
    Offline,
    Online,
    Busy,
    Error(String),
    Signal(String),
}

// type is locally contained
type PeerIdString = String;

// Must be serializable to send data across network
// issue with this is that this cannot be convert into Encode,Decode by bincode. Instead we'll have to
#[derive(Debug, Serialize, Deserialize)]
pub enum NodeEvent {
    // Hello(PeerIdString, ComputerSpec),
    Disconnected {
        peer_id: PeerIdString,
        reason: Option<String>,
    },
    BlenderStatus(BlenderEvent),
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

    pub async fn send_node_status(&mut self, status: NodeEvent) {
        if let Err(e) = self.sender.send(Command::NodeStatus(status)).await {
            eprintln!("Failed to send node status to network service: {e:?}");
        }
    }

    // send job event to all connected node
    pub async fn send_job_event(&mut self, event: JobEvent) {
        self.sender
            .send(Command::JobStatus(event))
            .await
            .expect("Command should not be dropped");
    }

    pub async fn file_service(&mut self, command: FileCommand) {
        self.sender
            .send(Command::FileService(command))
            .await
            .expect("Command should not have been dropped!");
    }

    /// file_name are broadcasted with the extensions included, but not the directory it's located in. E.g. "test.blend"
    // I need to use some kind of enumeration to help make this process flexible with rules..
    pub async fn start_providing(&mut self, provider: &ProviderRule) -> Result<(), NetworkError> {
        let cmd = match provider {
            ProviderRule::Default(path_buf) => {
                // TODO: remove .expect(), .to_str(), and .to_owned()
                match path_buf.file_name() {
                    Some(file_name) => {
                        let keyword = file_name
                            .to_str()
                            .expect("Must be able to convert OsStr to Str!");

                        FileCommand::StartProviding(keyword.into(), path_buf.into())
                    }
                    None => return Err(NetworkError::BadInput),
                }
            }
            ProviderRule::Custom(keyword, path_buf) => {
                FileCommand::StartProviding(keyword.to_owned(), path_buf.to_owned())
            }
        };

        if let Err(e) = self.sender.send(Command::FileService(cmd)).await {
            eprintln!("How did this happen? {e:?}");
        }
        Ok(())
    }

    pub async fn get_providers(&mut self, file_name: &str) -> Option<HashSet<PeerId>> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::FileService(FileCommand::GetProviders {
            file_name: file_name.to_string(),
            sender,
        });
        self.sender
            .send(cmd)
            .await
            .expect("Command receiver should not be dropped");

        receiver.await.unwrap_or(None)
    }

    // client request file from peers.
    // I feel like we should make this as fetching data from network? Some sort of stream?
    pub async fn get_file_from_peers<T: AsRef<Path>>(
        &mut self,
        file_name: &str,
        destination: T,
    ) -> Result<PathBuf, NetworkError> {
        let providers = self
            .get_providers(&file_name)
            .await
            .ok_or(NetworkError::NoPeerProviderFound)?;
        match providers.iter().next() {
            Some(peer_id) => {
                self.request_file(peer_id, file_name, destination.as_ref())
                    .await
            }
            None => Err(NetworkError::NoPeerProviderFound),
        }
    }

    async fn request_file(
        &mut self,
        peer_id: &PeerId,
        file_name: &str,
        destination: &Path,
    ) -> Result<PathBuf, NetworkError> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::FileService(FileCommand::RequestFile {
            peer_id: *peer_id,
            file_name: file_name.into(),
            sender,
        });
        self.sender
            .send(cmd)
            .await
            .expect("Command should not be dropped");
        let content = receiver
            .await
            .expect("Should not be closed?")
            .or_else(|e| Err(NetworkError::UnableToSave(e.to_string())))?;

        let file_path = destination.join(file_name);
        match async_std::fs::write(file_path.clone(), content).await {
            Ok(_) => Ok(file_path),
            Err(e) => Err(NetworkError::UnableToSave(e.to_string())),
        }
    }

    // TODO: Come back to this one and see how this one gets invoked.
    pub(crate) async fn respond_file(
        &mut self,
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    ) {
        let cmd = Command::FileService(FileCommand::RespondFile { file, channel });
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

    providing_files: HashMap<String, PathBuf>,
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<Option<HashSet<PeerId>>>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

// network service will be used to handle and receive network signal. It will also transmit network package over lan
impl NetworkService {
    pub fn new(
        swarm: Swarm<BlendFarmBehaviour>,
        receiver: Receiver<Command>,
        sender: Sender<Event>,
    ) -> NetworkService {
        Self {
            swarm,
            receiver,
            sender,
            providing_files: Default::default(),
            pending_get_providers: Default::default(),
            pending_request_file: Default::default(),
        }
    }

    /*
       From my understanding about this method implementation is that we wanted to be able to broadcast
       all of the potential files out there and sponsor what's available.
       I think this methodology will change because we wanted the host to ask the client if there's any files available
       or completed by this machine, and then reply back to the host.

       I need to setup a network diagram to make this network layer protocol clear and understand,
       as well as easy to debug, test, and identify potential issues.

       From the host side. the host will broadcast asking for job updates.
       This update will include job id.

       On the client side, the client will receive the notification from the host,
       and check the database to see if the job id exist.

       if it does exist, then the client will broadcast list of completed images.
       The host will receive this list and compare to the host machine to see if they have the image

       If the host does not have the image, it will initiate a file transfer between the host and the client machine
       In this case, we should not have to make all of the files available, but instead make the target image
       available for the host to transfer over the network protocol.

       This is recognized as a tcp handshake connection, asking for the image from the node
       and the node will send the image via channel request.
    */

    // here we will deviate handling the file service command.
    async fn process_file_service(&mut self, cmd: FileCommand) {
        match cmd {
            FileCommand::RequestFile {
                peer_id,
                file_name,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer_id, FileRequest(file_name.into()));
                self.pending_request_file.insert(request_id, sender);
            }
            FileCommand::RespondFile { file, channel } => {
                // somehow the send_response errored out? How come?
                // Seems like this function got timed out?
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, FileResponse(file))
                {
                    // why am I'm getting error message here?
                    eprintln!("Error received on sending response! {e:?}");
                }
            }
            FileCommand::GetProviders { file_name, sender } => {
                let key = file_name.into_bytes().into();
                let query_id = self.swarm.behaviour_mut().kad.get_providers(key);
                self.pending_get_providers.insert(query_id, sender);
            }
            FileCommand::StartProviding(keyword, file_path) => {
                let key = keyword.clone().into_bytes().into();
                // could we make use of this query ID?
                let _query_id = self
                    .swarm
                    .behaviour_mut()
                    .kad
                    .start_providing(key)
                    .expect("No store error.");
                self.providing_files.insert(keyword, file_path);
            }
            FileCommand::StopProviding(keyword) => {
                let key = RecordKey::new(&keyword.as_bytes());
                self.swarm.behaviour_mut().kad.stop_providing(&key);
                self.providing_files.remove(&keyword);
            }
            FileCommand::RequestFilePath { keyword, sender } => {
                let result = self
                    .providing_files
                    .get(&keyword)
                    .and_then(|f| Some(f.to_owned()));
                println!("{keyword:?} | {result:?}");
                sender.send(result).expect("Receiver should not be dropped");
            }
        };
    }

    // send command
    // Receive commands from foreign invocation.
    pub async fn process_command(&mut self, cmd: Command) {
        match cmd {
            Command::Status(msg) => {
                let topic = IdentTopic::new(STATUS);
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, msg.into_bytes())
                {
                    eprintln!("Fail to send status over network! {e:?}");
                }
            }
            Command::FileService(service) => self.process_file_service(service).await,
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
            // Send Job status to all network available.
            Command::JobStatus(event) => {
                // convert data into json format.
                let data = serde_json::to_string(&event).unwrap();
                let topic = IdentTopic::new(JOB.to_owned());
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Error sending job status! {e:?}");
                }
            }
            Command::NodeStatus(status) => {
                // we want to send this info across broadcast network. We do not care who is listening the network. Only the fact that we want our hosts to keep notify for availability.
                let data = serde_json::to_string(&status).unwrap();
                let topic = IdentTopic::new(NODE);
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
                        .expect("Request to still be pending")
                        .send(Ok(response.0));
                }
            },
            libp2p_request_response::Event::OutboundFailure {
                request_id, error, ..
            } => {
                let _ = self
                    .pending_request_file
                    .remove(&request_id)
                    .expect("Request to still be pending")
                    .send(Err(Box::new(error)));
            }
            libp2p_request_response::Event::ResponseSent { .. } => {}
            _ => {}
        }
    }

    async fn process_mdns_event(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, address) in peers {
                    println!("Discovered [{peer_id:?}] {address:?}");
                    // if I have already discovered this address, then I need to skip it. Otherwise I will produce garbage log input for duplicated peer id already exist.
                    // it seems that I do need to explicitly add the peers to the list.
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

    async fn process_gossip_event(&mut self, event: gossipsub::Event) {
        match event {
            // what is propagation source? can we use this somehow?
            gossipsub::Event::Message { message, .. } => match message.topic.as_str() {
                // if the topic is JOB related, assume data as JobEvent
                JOB => match serde_json::from_slice::<JobEvent>(&message.data) {
                    Ok(job_event) => {
                        // I don't think this function is called?
                        println!("Is this function used?");
                        if let Err(e) = self.sender.send(Event::JobUpdate(job_event)).await {
                            eprintln!("Something failed? {e:?}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Fail to parse Job topic data! {e:?}");
                    }
                },
                // Node based event awareness
                NODE => match serde_json::from_slice::<NodeEvent>(&message.data) {
                    Ok(node_event) => {
                        if let Err(e) = self.sender.send(Event::NodeStatus(node_event)).await {
                            eprintln!("Something failed? {e:?}");
                        }
                    }
                    Err(e) => eprintln!("fail to parse Node topic data! {e:?}"),
                },

                // Garbage collector - Treat this as a grain of salt. Do not execute any data from this scope
                // should only be used to display logs and info, things for us to identify unusual activity going on outside our domain specification.
                _ => {
                    // I received Mac.lan from message.topic?
                    let topic = message.topic.as_str();
                    eprintln!("Intercepted unhandled signal here: {topic}");
                }
            },
            // I should be logging info from other event from gossip... wonder what they got to say?
            // TODO: Log and verify if we need to handle other gossip events.
            _ => {}
        }
    }

    // async fn process_outbound_query(&mut )

    // Handle kademila events (Used for file sharing)
    // can we use this same DHT to make node spec publicly available?
    async fn process_kademlia_event(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed { id, result, .. } => {
                match result {
                    kad::QueryResult::StartProviding(providers) => {
                        println!("List of providers: {providers:?}");
                    }
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        providers,
                        ..
                    })) => {
                        // So, here's where we finally receive the invocation?
                        if let Some(sender) = self.pending_get_providers.remove(&id) {
                            sender
                                .send(Some(providers.clone()))
                                .expect("Receiver not to be dropped");

                            if let Some(mut node) = self.swarm.behaviour_mut().kad.query_mut(&id) {
                                node.finish();
                            }
                        }
                    }
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                    )) => {
                        if let Some(sender) = self.pending_get_providers.remove(&id) {
                            sender.send(None).expect("Sender not to be dropped");
                        }

                        if let Some(mut node) = self.swarm.behaviour_mut().kad.query_mut(&id) {
                            node.finish();
                        }
                        // This piece of code means that there's nobody advertising this on the network?
                        // what was suppose to happen here?
                        // TODO: I am once again stopped here. This message appeared from the CLI side. Not the host.

                        // let outbound_request_id = id;
                        // let event = Event::PendingRequestFiled(outbound_request_id, None);
                        // self.sender.send(event).await;
                    }
                    kad::QueryResult::PutRecord(result) => match result {
                        Ok(value) => println!("Successfully append the record! {value:?}"),
                        Err(e) => eprintln!("Error putting record in! {e:?}"),
                    },
                    // suppressed
                    _ => {}
                }
            }

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

    // Process incoming network events - Treat this as receiving new orders.
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
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                println!("Connection Established: {peer_id:?}\n{endpoint:?}");
                // once we establish a connection, we should ping kademlia for all available nodes on the network.
                // let key = NODE.to_vec();
                // let _query_id = self.swarm.behaviour_mut().kad.get_providers(key.into());

                // let mut machine = Machine::new();
                // let spec = ComputerSpec::new(&mut machine);
                // let event = Event::NodeStatus(NodeEvent::Discovered(spec));
                // if let Err(e) = self.sender.send(event).await {
                //     eprintln!("Fail to send event on connection established! {e:?}");
                // }
            }
            // This was called when client starts while manager is running. "Connection error: I/O error: closed by peer: 0"
            // TODO: Read what ConnectionClosed does?
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                let reason = cause.and_then(|f| Some(f.to_string()));
                let node = NodeEvent::Disconnected {
                    peer_id: peer_id.to_base58(),
                    reason,
                };
                let event = Event::NodeStatus(node);
                if let Err(e) = self.sender.send(event).await {
                    eprintln!("Fail to send event on connection closed! {e:?}");
                }
            }
            // TODO: Figure out what these events are, and see if they're any use for us to play with or delete them. Unnecessary comment codeblocks
            // SwarmEvent::ListenerClosed { .. } => todo!(),
            // SwarmEvent::ListenerError { listener_id, error } => todo!(),
            // vv ignore events below vv
            SwarmEvent::NewListenAddr { .. } => {
                // println!("[New Listener Address]: {address}");
            }
            // SwarmEvent::Dialing { .. } => {} // Suppressing logs
            // SwarmEvent::IncomingConnection { .. } => {} // Suppressing logs
            // SwarmEvent::NewExternalAddrOfPeer { .. } => {}
            // SwarmEvent::OutgoingConnectionError { connection_id, peer_id, error } => {}  // I recognize this and do want to display result below.
            // SwarmEvent::IncomingConnectionError { .. } => {}                             // I recognize this and do want to display result below.

            // ^^eof ignore^^
            // we'll do nothing for this for now.
            // see what we're skipping? Anything we identify must have described behaviour, or add to ignore list.
            _ => {
                println!("[Network]: {event:?}");
            }
        };
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                msg = self.receiver.select_next_some() => self.process_command(msg).await,
                event = self.swarm.select_next_some() => self.process_swarm_event(event).await,
            }
        }
    }
}
