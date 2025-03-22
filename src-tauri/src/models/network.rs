use super::behaviour::{BlendFarmBehaviour, FileResponse, FileService};
use super::job::JobEvent;
use super::message::{NetCommand, NetEvent, NetworkError};
use super::server_setting::ServerSetting;
use core::str;
use std::sync::Arc;
use futures::{channel::oneshot, prelude::*};
use libp2p::gossipsub;
use libp2p::{
    kad, mdns, ping,
    swarm::Swarm,
    tcp, Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use libp2p_request_response::{ProtocolSupport, ResponseChannel};
use machine_info::Machine;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use std::u64;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::{io, join /*, select */};

/*
Network Service - Receive, handle, and process network request. 
*/

pub const STATUS: &str = "blendfarm/status";
pub const SPEC: &str = "blendfarm/spec";
pub const JOB: &str = "blendfarm/job";
pub const HEARTBEAT: &str = "blendfarm/heartbeat";
const TRANSFER: &str = "/file-transfer/1";

// the tuples return two objects
// Network Controller invokes network commands
// Receiver<NetCommand> receive network events
pub async fn new() -> Result<(NetworkController, Receiver<NetEvent>), NetworkError>
{
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

    let network_service = NetworkService {
        swarm,
        receiver,
        sender: event_sender,
        public_addr: None,
        machine: Machine::new(),
        pending_dial: Default::default(),
        // TODO: job_service
        // pending_task: Default::default(),
    };

    // start network service async
    let thread = tokio::spawn(network_service.run(&mut receiver, &mut event_sender));

    Ok((
        NetworkController {
            sender,
            settings: ServerSetting::load(),
            providing_files: Default::default(),
            // there could be some other factor this this may not work as intended? Let's find out soon!
            public_id,
            hostname: Machine::new().system_info().hostname,
            thread
        },
        event_receiver,
    ))
}

// where is this used? Can we use this for network services?
// why do I need to clone this?
pub struct NetworkController {
    // send net commands
    sender: mpsc::Sender<NetCommand>,

    // contain server settings...? Questionable? Dependency coupling?
    pub settings: ServerSetting,

    // move this to file_service?
    // Use string to defer OS specific path system. This will be treated as a URI instead. /job_id/frame
    pub providing_files: HashMap<String, PathBuf>,
    
    // making it public until we can figure out how to use it correctly.
    pub public_id: PeerId,
    
    // must have this available somewhere.
    // Can we make this private?
    pub hostname: String,

    // network service background thread
    thread: JoinHandle<()>,
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
        self.providing_files.insert(file_name.clone(), path);
        println!("Start providing file {:?}", &file_name);
        let cmd = NetCommand::StartProviding { file_name, sender };
        self.sender
            .send(cmd)
            .await
            .expect("Command receiver not to be dropped");
        receiver.await.expect("Sender should not be dropped");
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
        receiver.await.expect("Sender should not be dropped")
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
            None => return Err(NetworkError::NoPeerProviderFound)
        };

        match content {
            Ok(content) => {
                let file_path = destination.join(file_name);
                match async_std::fs::write(file_path.clone(), content).await {
                    Ok(_) => Ok(file_path),
                    Err(e) => Err(NetworkError::UnableToSave(e.to_string())),
                }
            },
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
        receiver
            .await
            .expect("Command receiver should not be dropped")
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
        receiver.await.expect("Sender should not be dropped")
    }

    pub(crate) async fn respond_file(
        &mut self,
        file: Vec<u8>,
        channel: ResponseChannel<FileResponse>,
    ) {
        let cmd = NetCommand::RespondFile { file, channel };
        self.sender
            .send(cmd)
            .await
            .expect("Command should not be dropped");
    }
}

// this will help launch libp2p network. Should use QUIC whenever possible!
pub struct NetworkService {
    // swarm behaviour - interface to the network
    swarm: Swarm<BlendFarmBehaviour>,

    // receive Network command
    receiver: Receiver<NetCommand>,
    
    // Send Network event to subscribers.
    sender: Sender<NetEvent>,

    // Used to collect computer basic hardware info to distribute
    machine: Machine,

    public_addr: Option<Multiaddr>,
    
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
    // feels like we got a coupling nightmare here?
    // pending_task: HashMap<PeerId, oneshot::Sender<Result<Task, Box<dyn Error + Send>>>>,
}

// network service will be used to handle and receive network signal. It will also transmit network package over lan
impl NetworkService {

    pub fn get_host_name(&mut self) -> String {
        self.machine.system_info().hostname
    }

    // when I run, this will continue to run indefinitely
    pub async fn run(&mut self, cmd: &mut Receiver<NetCommand>, sender: Sender<NetEvent>) {
        

        let b1 = Arc::new(RwLock::new(self.swarm.behaviour_mut()));
        let b2 = b1.clone();
        let fs1 = Arc::new(RwLock::new(FileService::new()));
        let fs2 = fs1.clone();

        // should have a channel here to send command in between?
        let cmd_loop = tokio::spawn( async move {
            for cmd in cmd.recv().await {
                let mut file_service = fs1.write().await;
                let mut behaviour = b1.write().await;
                &mut behaviour.handle_command( &mut file_service, cmd ).await;
            }
        });

        // can't I just handle the stream from swarm? That way I can avoid this entirely?
        let net_loop = tokio::spawn(async move {
            loop {
                if let Some(event) = &self.swarm.next().await {
                    let mut file_service = fs2.write().await;
                    let mut behaviour = b2.write().await;
                    &mut behaviour.handle_event(&mut sender, &mut file_service, &event).await;
                }
            }
        });
        
        // how do I gracefully abort?
        join!(cmd_loop, net_loop);
    }
}

// impl AsRef<Receiver<NetCommand>> for NetworkService {
//     fn as_ref(&self) -> &Receiver<NetCommand> {
//         &self.command_receiver
//     }
// }
