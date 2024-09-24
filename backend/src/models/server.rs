use super::message::{
    CmdMessage,
    Destination::{self, All, Target},
    NetResponse,
};
use crate::models::{job::Job, message::NetMessage};
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, NodeTask, StoredNetEvent, StoredNodeEvent};
#[cfg(target_family="windows")]
use std::os::windows::fs::MetadataExt;
#[cfg(target_family="unix")]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    net::SocketAddr,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::mpsc::{self},
    thread,
    time::Duration,
};
use std::{os::unix::fs::MetadataExt, path::Path}; // hmm I'm concern about this one? Why is this any different than regular fs::metadata?

pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";
const INTERVAL_MS: u64 = 500;

// Issue: Cannot derive debug because NodeTask doesn't derive Debug! Omit NodeTask if you need to Debug!
// TODO: provide documentation explaining what this function does.
/// A server struct that holds the receiver transmission of Net Responses from other clients on the network, and a thread to process and run network packets in the background.
pub struct Server {
    tx: mpsc::Sender<CmdMessage>,
    pub rx_recv: Option<mpsc::Receiver<NetResponse>>,
    _task: NodeTask,
}

impl Server {
    fn generate_ping(socket: &SocketAddr) -> NetMessage {
        NetMessage::Ping {
            server_addr: Some(socket.to_owned()),
        }
    }

    // wonder do I need to make this return an actual raw mutable pointer to heap?
    pub fn new(port: u16) -> Server {
        let (handler, listener) = node::split::<NetMessage>();

        let (_task, mut receiver) = listener.enqueue();
        let public_addr =
            SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), port);

        // listen tcp
        handler
            .network()
            .listen(Transport::FramedTcp, public_addr)
            .unwrap();

        // connect udp
        let (udp_conn, _) = match handler.network().connect(Transport::Udp, MULTICAST_ADDR) {
            Ok(data) => data,
            Err(e) => {
                panic!("{e}");
            }
        };

        // listen udp
        handler
            .network()
            .listen(Transport::Udp, MULTICAST_ADDR)
            .unwrap();

        // this is starting to feel like event base driven programming?
        // is this really the best way to handle network messaging?
        let (tx, rx) = mpsc::channel();
        let (tx_recv, rx_recv) = mpsc::channel();

        // I wonder if there's a way to simply this implementation code?
        // it would be nice if I could just provide a callback function to poll this?
        thread::spawn(move || {
            let mut peers: HashSet<Endpoint> = HashSet::new();
            // TODO: Find a better place for this?
            let current_job: Option<Job> = None;

            loop {
                // seems like a hack?
                // TODO: Find a better way to handle this?
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        CmdMessage::SendJob(job) => {
                            // send new job to all clients
                            let info = &NetMessage::SendJob(job);
                            // send to all connected clients on udp channel
                            for peer in peers.iter() {
                                handler.network().send(*peer, &info.serialize());
                            }
                        }
                        CmdMessage::SendFile(file_path, destination) => {
                            let mut file = File::open(&file_path).unwrap();
                            let file_name = file_path.file_name().unwrap().to_str().unwrap();
                            // let size = file.metadata().unwrap().size() as usize;
                            let size = file.metadata().unwrap().file_size() as usize;
                            let mut data: Vec<u8> = Vec::with_capacity(size);
                            let bytes = file.read_to_end(&mut data).unwrap();
                            if bytes != size {
                                println!("Warning! File might be corrupted! read size not the same as file size!");
                            }
                            let msg = NetMessage::SendFile(file_name.to_owned(), data).serialize();

                            match destination {
                                Target(target) => {
                                    handler.network().send(target, &msg);
                                }
                                All => {
                                    for peer in peers.iter() {
                                        handler.network().send(*peer, &msg);
                                    }
                                }
                            };
                        }
                        CmdMessage::AddPeer { name, socket } => {
                            // hmm wonder what this'll do?
                            match handler.network().connect(Transport::FramedTcp, socket) {
                                Ok((peer, _)) => {
                                    println!("Connected to peer `{}` [{}]", name, peer.addr());
                                    peers.insert(peer);
                                }
                                Err(e) => {
                                    println!("Error connecting to peer! {}", e);
                                }
                            }
                        }
                        CmdMessage::Ping => {
                            // send ping to all clients
                            handler
                                .network()
                                .send(udp_conn, &Self::generate_ping(&public_addr).serialize());
                        }
                        CmdMessage::AskForBlender { version } => {
                            // send out a request to all clients to check for blender version
                            let info = &NetMessage::CheckForBlender {
                                os: std::env::consts::OS.to_owned(),
                                version,
                                arch: std::env::consts::ARCH.to_owned(),
                                caller: public_addr,
                            };
                            for peer in peers.iter() {
                                handler.network().send(*peer, &info.serialize());
                            }
                        }
                        CmdMessage::Exit => {
                            // Wonder why I can't see this? Does it not stop?
                            println!("Terminate signal received!");
                            handler.stop();
                            break;
                        }
                        CmdMessage::GetPeers => {
                            if let Err(e) = tx_recv.send(NetResponse::PeerList {
                                addrs: peers.clone(),
                            }) {
                                println!("Fail to send notification back to subscribers\n{e}");
                            }
                        }
                    }
                }

                // check and process network events
                if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                    match event {
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let msg = match NetMessage::deserialize(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    println!("Error deserializing net message data! \n{e}");
                                    continue;
                                }
                            };

                            println!("Message received from [{}]\n{:?}", endpoint.addr(), &msg);

                            // I wouldn't imagine having broken/defragmented packets within local network?
                            match msg {
                                // I'm working on something I don't know if it'll work or not...
                                // this I can omit, but how do I make this code better?
                                NetMessage::CheckForBlender { caller, .. } => {
                                    // omit the caller from the list of peers
                                    for peer in peers.iter().filter(|p| p.addr() != caller) {
                                        handler.network().send(*peer, &msg.serialize());
                                    }
                                }
                                NetMessage::Ping { server_addr: None } => {
                                    // we should not attempt to connect to the host!
                                    println!(
                                        "Received ping from client! Sending broadcast signal!"
                                    );

                                    // maybe I should just send out a server ping signal instead?
                                    handler.network().send(
                                        udp_conn,
                                        &Self::generate_ping(&public_addr).serialize(),
                                    );
                                }
                                NetMessage::Ping {
                                    server_addr: Some(_),
                                } => { /* Server should ignore other server ping */ }
                                NetMessage::SendJob(job) => {
                                    println!("Received job from [{}]\n{:?}", endpoint.addr(), job);
                                    tx_recv.send(NetResponse::JobSent(job)).unwrap();
                                    // current_job = Some(job);
                                }
                                NetMessage::RequestJob => {
                                    // at this point here, client is asking us for a new job.
                                    if let Some(ref job) = current_job {
                                        let job = job.clone();
                                        handler
                                            .network()
                                            .send(endpoint, &NetMessage::SendJob(job).serialize());
                                    } else {
                                        println!("No job available to send!");
                                    }
                                }
                                _ => println!("Unhandled case for {:?}", msg),
                            }
                        }
                        StoredNetEvent::Connected(endpoint, _) => {
                            // we connected via udp channel!
                            if endpoint == udp_conn {
                                println!("Connected via UDP channel! [{}]", endpoint.addr());
                                let msg = Self::generate_ping(&public_addr);
                                handler.network().send(endpoint, &msg.serialize());
                            }
                            // we connected via tcp channel!
                            else {
                                println!("Connected via TCP channel! [{}]", endpoint.addr());
                            }
                        }
                        StoredNetEvent::Accepted(endpoint, _) => {
                            println!("Server accepts connection: [{}]", endpoint);
                            peers.insert(endpoint);
                            tx_recv
                                .send(NetResponse::Joined {
                                    socket: endpoint.addr(),
                                })
                                .unwrap();
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            println!("Disconnected event receieved! [{}]", endpoint.addr());
                            if peers.remove(&endpoint) {
                                tx_recv
                                    .send(NetResponse::Disconnected {
                                        socket: endpoint.addr(),
                                    })
                                    .unwrap();
                                println!("[{}] has disconnected", endpoint.addr());
                            }
                        }
                    }
                }
            }
        });

        Self {
            tx,
            rx_recv: Some(rx_recv),
            _task,
        }
    }

    pub fn send_job(&self, job: Job) {
        if let Err(e) = self.tx.send(CmdMessage::SendJob(job)) {
            println!("Issue sending job request to server! {e}");
        }
    }

    /// Send a file to all network nodes.
    pub fn send_file(&self, file_path: impl AsRef<Path>) {
        let file_path = file_path.as_ref();
        if !file_path.is_file() {
            println!("file path is not a file! Aborting operation!");
            return;
        }

        if let Err(e) = self
            .tx
            .send(CmdMessage::SendFile(file_path.to_owned(), Destination::All))
        {
            println!("Failed to send file request to server! {e}");
        }
    }

    pub fn get_peer_list(&self) {
        if let Err(e) = self.tx.send(CmdMessage::GetPeers) {
            println!("Unable to send server command message!\n{e}");
        }
    }

    #[allow(dead_code)]
    pub fn send_file_to_target(&self, file_path: &PathBuf, target: Endpoint) {
        if let Err(e) = self
            .tx
            .send(CmdMessage::SendFile(file_path.to_owned(), Target(target)))
        {
            println!("Failed to send file request to server! {e}");
        }
    }

    pub fn ping(&self) {
        self.tx.send(CmdMessage::Ping).unwrap();
    }

    pub fn connect(&self, name: &str, socket: SocketAddr) {
        self.tx
            .send(CmdMessage::AddPeer {
                name: name.to_owned(),
                socket,
            })
            .unwrap();
    }

    // going to have to put a hold on this for now...
    // code may not be useful anymore. may have to refactor this to make this function working again.
    /*
    /// A client request if other client have identical blender version
    fn check_for_blender(
        endpoint: Endpoint,
        os: &str,
        arch: &str,
        version: &Version,
    ) -> NetMessage {
        let current_os = std::env::consts::OS;
        let current_arch = std::env::consts::ARCH;
        let default_msg = NetMessage::HaveMatchingBlenderRequirement {
            have_blender: false,
        };

        println!(
            "Client [{}] have asked me if I have matching blender? OS: {} | Arch: {} | Version: {}",
            endpoint.addr(),
            os,
            arch,
            version
        );

        match (os, arch) {
            (os, arch) if current_os.eq(os) & current_arch.eq(arch) => {
                let manager = BlenderManager::load();
                let blender = manager.have_blender(&version);
                let msg = NetMessage::HaveMatchingBlenderRequirement {
                    have_blender: blender,
                };
                msg
            }
            (os, arch) if current_os.eq(os) => {
                println!(
                    "Client [{}] have incompatible Arch, ignoring! {}(Client) != {}(Target))",
                    endpoint.addr(),
                    arch,
                    current_arch
                );
                default_msg
            }
            (os, _) => {
                println!(
                    "Client [{}] have incompatible OS, ignoring! {}(Client) != {}(Target)",
                    endpoint.addr(),
                    os,
                    current_os
                );
                default_msg
            }
        }

        // in this case, the client is asking all other client if any one have the matching blender version type.
        // let _map = self
        //     .nodes
        //     .iter()
        //     .filter(|n| n.endpoint != endpoint)
        //     .map(|n| self.send_to_target(n.endpoint, &msg));
    }
    */
}

impl Drop for Server {
    fn drop(&mut self) {
        println!("Dropping Server struct!");
        self.tx.send(CmdMessage::Exit).unwrap();
    }
}
