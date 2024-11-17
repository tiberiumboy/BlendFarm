use super::message::{FromNetwork, NetMessage, NetResponse, ToNetwork};
use crate::models::job::Job;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, NodeTask, StoredNetEvent, StoredNodeEvent};
use std::net::{IpAddr, Ipv4Addr};
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;
use std::{
    collections::HashSet,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

const INTERVAL_MS: u64 = 350;

// Issue: Cannot derive debug because NodeTask doesn't derive Debug! Omit NodeTask if you need to Debug!
// TODO: provide documentation explaining what this function does.
/// A server struct that holds the receiver transmission of Net Responses from other clients on the network, and a thread to process and run network packets in the background.
// TODO: I need to either find a way to change the state of this struct once connected.
// otherwise we're back to this problem again?
pub struct Server {
    tx: Arc<Sender<ToNetwork>>,
    pub rx_recv: Arc<Receiver<NetResponse>>,
    task: NodeTask,
    addr: SocketAddr,
}

impl Server {
    // wonder do I need to make this return an actual raw mutable pointer to heap?
    pub fn new(port: u16) -> Server {
        let (handler, listener) = node::split::<NetMessage>();

        let (task, mut receiver) = listener.enqueue();

        let public_addr =
            SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), port);

        // move this into separate method implementation
        // listen tcp
        handler
            .network()
            .listen(Transport::FramedTcp, public_addr)
            .unwrap();

        // this is starting to feel like event base driven programming?
        // is this really the best way to handle network messaging?
        let (tx, rx) = mpsc::channel();
        let (tx_recv, rx_recv) = mpsc::channel();

        // I wonder if there's a way to simply this implementation code?
        thread::spawn(move || {
            // Move this method out of this function implementation.
            // TODO: Find a better place for this?
            // let current_job: Option<Job> = None;
            let mut peers: HashSet<Endpoint> = HashSet::new();

            loop {
                // TODO: Find a better way to handle this?
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));
                // TODO: relocate this elsewhere?
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        ToNetwork::Connect(addr) => {
                            match handler.network().connect(Transport::FramedTcp, addr) {
                                // why can't we get a message saying "Welcome?"
                                Ok((peer, _)) => {
                                    println!("[{}] Connected", peer.addr());
                                    peers.insert(peer);
                                    // TODO: We'll come back to this one again.
                                    // tx_recv.send(NetResponse::{ socket: peer.addr() });
                                }
                                Err(e) => {
                                    println!("Error connecting to peer! {}", e);
                                }
                            };
                        }
                        ToNetwork::SendFile(file_name, data) => {}
                        ToNetwork::Ping => {
                            // Should not be called from this level!
                        }
                        ToNetwork::GetPeers => tx_recv
                            .send(NetResponse::PeerList {
                                addrs: peers.clone(),
                            })
                            .unwrap(),

                        // match msg {
                        //     // CmdMessage::SendJob(job) => {
                        //     //     // send new job to all clients
                        //     //     let info = &NetMessage::SendJob(job);
                        //     //     // send to all connected clients on udp channel
                        //     //     for peer in peers.iter() {
                        //     //         handler.network().send(*peer, &info.serialize());
                        //     //     }
                        //     // }
                        //     NetMessage::SendFile(file_path, destination) => {
                        //         let mut file = File::open(&file_path).unwrap();
                        //         let file_name = file_path.file_name().unwrap().to_str().unwrap();
                        //         #[cfg(target_family = "windows")]
                        //         let size = file.metadata().unwrap().file_size() as usize;
                        //         #[cfg(target_family = "unix")]
                        //         let size = file.metadata().unwrap().size() as usize;
                        //         let mut data: Vec<u8> = Vec::with_capacity(size);
                        //         let bytes = file.read_to_end(&mut data).unwrap();
                        //         if bytes != size {
                        //             println!("Warning! File might be corrupted! read size not the same as file size!");
                        //         }
                        //         let msg = FromNetwork::SendFile(file_name.to_owned(), data).serialize();

                        //         // try to figure out what I did here?
                        //         match destination {
                        //             Target(target) => {
                        //                 handler.network().send(target, &msg);
                        //             }
                        //             All => {
                        //                 for peer in peers.iter() {
                        //                     handler.network().send(*peer, &msg);
                        //                 }
                        //             }
                        //         };
                        //     }
                        //     NetMessage::Ping => {
                        //         // send ping to all clients
                        //         let ping = FromNetwork::Ping {
                        //             server_addr: Some(public_addr.to_owned()),
                        //         };
                        //         handler.network().send(udp_conn, &ping.serialize());
                        //     }
                        //     ToNetwork::AskForBlender { version } => {
                        // //         // send out a request to all clients to check for blender version
                        //         let info = &FromNetwork::CheckForBlender {
                        //             os: std::env::consts::OS.to_owned(),
                        //             version,
                        //             arch: std::env::consts::ARCH.to_owned(),
                        //             caller: public_addr,
                        //         };
                        //         for peer in peers.iter() {
                        //             handler.network().send(*peer, &info.serialize());
                        //         }
                        //     }
                        ToNetwork::Exit => {
                            // Wonder why I can't see this? Does it not stop?
                            println!("Terminate signal received!");
                            handler.stop();
                            break;
                        } //     NetMessage::GetPeers => {
                          //         if let Err(e) = tx_recv.send(NetResponse::PeerList {
                          //             addrs: peers.clone(),
                          //         }) {
                          //             // should there be a log about this? What's the expected behaviour from this?
                          //             // TODO: we should do something about this..
                          //             println!("Fail to send notification back to subscribers\n{e}");
                          //         }
                          //     }
                          // }
                    }
                }

                // check and process network events
                if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                    match event {
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let msg = match NetMessage::de(&bytes) {
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
                                // NetMessage::CheckForBlender { caller, .. } => {
                                //     // omit the caller from the list of peers
                                //     for peer in peers.iter().filter(|p| p.addr() != caller) {
                                //         handler.network().send(*peer, &msg.serialize());
                                //     }
                                // }
                                NetMessage::Ping(None) => {
                                    // we should not attempt to connect to the host!
                                    println!(
                                        "Received ping from client! Sending broadcast signal!"
                                    );
                                    let ping = NetResponse::Ping;
                                    tx_recv.send(ping).unwrap();
                                }
                                NetMessage::Ping(Some(_)) => {
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
                                NetMessage::SendFile(file_name, data) => {
                                    let path = std::env::temp_dir().join(file_name);
                                    std::fs::write(&path, data).unwrap();
                                    tx_recv.send(NetResponse::FileTransfer(path)).unwrap();
                                }
                            }
                        }
                        StoredNetEvent::Connected(endpoint, _) => {
                            // we connected via udp channel!
                            if endpoint == udp_conn {
                                println!("Connected via UDP channel! [{}]", endpoint.addr());
                                let msg = FromNetwork::Ping {
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
                        } // TODO: Would be nice to stream bits of file information instead?
                          // for now make this working again
                          // StoredNodeEvent::Signal(signal) => match signal {
                          //     Signal::Stream(data) => {
                          //         if let Some((data, start, length)) => data {

                          //             handler.signals().send_with_timer(Signal::Stream)
                          //         } else {
                          //             println!("File sent!");
                          //             handler.stop();
                          //         }
                          //     }
                          // }
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
    pub fn send_file(&self, file: impl AsRef<Path>) {
        let file_path = file.as_ref();
        if !file_path.is_file() {
            println!("file path is not a file! Aborting operation!");
            return;
        }

        let data = std::fs::read(file_path).unwrap();
        let file_name = file_path.file_name().unwrap().to_str().unwrap();
        if let Err(e) = self
            .tx
            .send(ToNetwork::SendFile(file_name.to_owned(), data))
        {
            println!("Failed to send file request to server! {e}");
        }
    }

    pub fn get_peer_list(&self) {
        if let Err(e) = self.tx.send(ToNetwork::GetPeers) {
            println!("Unable to send server command message!\n{e}");
        }
    }

    pub fn connect_peer(&self, socket: SocketAddr) {
        self.tx.send(ToNetwork::Connect(socket)).unwrap();
    }
}

impl AsRef<SocketAddr> for Server {
    fn as_ref(&self) -> &SocketAddr {
        &self.addr
    }
}

// impl Drop for Server {
//     fn drop(&mut self) {
//         println!("Dropping Server struct!");
//         self.tx.send(CmdMessage::Exit).unwrap();
//     }
// }

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
