use super::message::{
    CmdMessage,
    Destination::{self, All, Target},
    NetMessage, NetResponse,
};

use crate::models::job::Job;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, NodeTask, StoredNetEvent, StoredNodeEvent};
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

// Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
// pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";
pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
pub const MULTICAST_PORT: u16 = 3010;
pub const MULTICAST_SOCK: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_ADDR), MULTICAST_PORT);
const INTERVAL_MS: u64 = 350;

// Issue: Cannot derive debug because NodeTask doesn't derive Debug! Omit NodeTask if you need to Debug!
// TODO: provide documentation explaining what this function does.
/// A server struct that holds the receiver transmission of Net Responses from other clients on the network, and a thread to process and run network packets in the background.
// TODO: I need to either find a way to change the state of this struct once connected.
// otherwise we're back to this problem again?
pub struct Server {
    tx: Sender<CmdMessage>,
    pub rx_recv: Arc<Receiver<NetResponse>>,
    task: NodeTask,
    addr: SocketAddr,
}

impl Server {
    // wonder do I need to make this return an actual raw mutable pointer to heap?
    pub fn new(port: u16) -> Server {
        let (handler, listener) = node::split::<NetMessage>();

        let (task, mut receiver) = listener.enqueue();
        // was this design to handle computer off the network?
        let public_addr =
            SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), port);

        // move this into separate method implementation
        // listen tcp
        handler
            .network()
            .listen(Transport::FramedTcp, public_addr)
            .unwrap();

        // connect udp
        let (udp_conn, _) = match handler.network().connect(Transport::Udp, MULTICAST_SOCK) {
            Ok(data) => data,
            // Err(e) if e.kind() == std::io::ErrorKind::NetworkUnreachable => {
            //     todo!("how can we gracefully tell the program to continue to run on local network - despite we're \"offline\"?");
            // }
            Err(e) => panic!("{e}"),
        };

        // listen udp
        handler
            .network()
            .listen(Transport::Udp, MULTICAST_SOCK)
            .unwrap();

        // this is starting to feel like event base driven programming?
        // is this really the best way to handle network messaging?
        let (tx, rx) = mpsc::channel();
        let (tx_recv, rx_recv) = mpsc::channel();

        // I wonder if there's a way to simply this implementation code?
        thread::spawn(move || {
            let mut peers: HashSet<Endpoint> = HashSet::new();
            // Move this method out of this function implementation.
            // TODO: Find a better place for this?
            // let current_job: Option<Job> = None;

            loop {
                // TODO: Find a better way to handle this?
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));
                // TODO: relocate this elsewhere?
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        // CmdMessage::SendJob(job) => {
                        //     // send new job to all clients
                        //     let info = &NetMessage::SendJob(job);
                        //     // send to all connected clients on udp channel
                        //     for peer in peers.iter() {
                        //         handler.network().send(*peer, &info.serialize());
                        //     }
                        // }
                        CmdMessage::SendFile(file_path, destination) => {
                            let mut file = File::open(&file_path).unwrap();
                            let file_name = file_path.file_name().unwrap().to_str().unwrap();
                            #[cfg(target_family = "windows")]
                            let size = file.metadata().unwrap().file_size() as usize;
                            #[cfg(target_family = "unix")]
                            let size = file.metadata().unwrap().size() as usize;
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
                            let ping = NetMessage::Ping {
                                server_addr: Some(public_addr.to_owned()),
                            };
                            handler.network().send(udp_conn, &ping.serialize());
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

                                    let ping = NetMessage::Ping {
                                        server_addr: Some(public_addr.to_owned()),
                                    };
                                    handler.network().send(udp_conn, &ping.serialize());
                                }
                                NetMessage::Ping {
                                    server_addr: Some(_),
                                } => {
                                    println!(
                                        "Received server ping, but we're the server? Ignoring"
                                    );
                                }
                                // NetMessage::SendJob(job) => {
                                //     println!("Received job from [{}]\n{:?}", endpoint.addr(), job);
                                //     tx_recv.send(NetResponse::JobSent(job)).unwrap();
                                //     // current_job = Some(job);
                                // }
                                NetMessage::RequestJob => {
                                    todo!("Impl. Requesting job");
                                    // at this point here, client is asking us for a new job.
                                    // if let Some(ref job) = current_job {
                                    //     let job = job.clone();
                                    //     handler
                                    //         .network()
                                    //         .send(endpoint, &NetMessage::SendJob(job).serialize());
                                    // } else {
                                    //     println!("No job available to send!");
                                    // }
                                }
                                _ => println!("Unhandled case for {:?}", msg),
                            }
                        }
                        StoredNetEvent::Connected(endpoint, _) => {
                            // we connected via udp channel!
                            if endpoint == udp_conn {
                                println!("Connected via UDP channel! [{}]", endpoint.addr());
                                let msg = NetMessage::Ping {
                                    server_addr: Some(public_addr.to_owned()),
                                };
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
            rx_recv: Arc::new(rx_recv),
            task,
            addr: public_addr,
        }
    }

    // TODO: repurpose this so that we're sending through NetworkService instead of interfacing to Server directly.
    pub fn send_job(&self, _job: Job) {
        // if let Err(e) = self.tx.send(CmdMessage::SendJob(job)) {
        //     println!("Issue sending job request to server! {e}");
        // }
    }

    /// Send a file to all network nodes.
    #[allow(dead_code)]
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
}

// impl NetworkNode for Server {
//     fn listen(&mut self) {
//         println!("Listening");
//     }

//     fn stop(&mut self) {
//         self._task.wait();
//     }

//     fn ping(&self) {
//         // it would be nice to be able to send a message to the handler instead? But I think this is because of the design limitation
//         // we're running the network handler in a background loop... How do I send a command to send ping out?
//         self.tx.send(CmdMessage::Ping).unwrap();
//     }

//     // don't think I need this anymore?
//     // pub fn connect(&self, name: &str, socket: SocketAddr) {
//     //     self.tx
//     //         .send(CmdMessage::AddPeer {
//     //             name: name.to_owned(),
//     //             socket,
//     //         })
//     //         .unwrap();
//     // }

//     // going to have to put a hold on this for now...
//     // code may not be useful anymore. may have to refactor this to make this function working again.
//     /*
//     /// A client request if other client have identical blender version
//     fn check_for_blender(
//         endpoint: Endpoint,
//         os: &str,
//         arch: &str,
//         version: &Version,
//     ) -> NetMessage {
//         let current_os = std::env::consts::OS;
//         let current_arch = std::env::consts::ARCH;
//         let default_msg = NetMessage::HaveMatchingBlenderRequirement {
//             have_blender: false,
//         };

//         println!(
//             "Client [{}] have asked me if I have matching blender? OS: {} | Arch: {} | Version: {}",
//             endpoint.addr(),
//             os,
//             arch,
//             version
//         );

//         match (os, arch) {
//             (os, arch) if current_os.eq(os) & current_arch.eq(arch) => {
//                 let manager = BlenderManager::load();
//                 let blender = manager.have_blender(&version);
//                 let msg = NetMessage::HaveMatchingBlenderRequirement {
//                     have_blender: blender,
//                 };
//                 msg
//             }
//             (os, arch) if current_os.eq(os) => {
//                 println!(
//                     "Client [{}] have incompatible Arch, ignoring! {}(Client) != {}(Target))",
//                     endpoint.addr(),
//                     arch,
//                     current_arch
//                 );
//                 default_msg
//             }
//             (os, _) => {
//                 println!(
//                     "Client [{}] have incompatible OS, ignoring! {}(Client) != {}(Target)",
//                     endpoint.addr(),
//                     os,
//                     current_os
//                 );
//                 default_msg
//             }
//         }

//         // in this case, the client is asking all other client if any one have the matching blender version type.
//         // let _map = self
//         //     .nodes
//         //     .iter()
//         //     .filter(|n| n.endpoint != endpoint)
//         //     .map(|n| self.send_to_target(n.endpoint, &msg));
//     }
//     */
// }

// impl Drop for Server {
//     fn drop(&mut self) {
//         println!("Dropping Server struct!");
//         self.tx.send(CmdMessage::Exit).unwrap();
//     }
// }
