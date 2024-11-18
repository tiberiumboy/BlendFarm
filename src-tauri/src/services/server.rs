/*
    Combine this with client
 */
use super::message::{ NetMessage, ToNetwork};
use super::network_service::NetworkNode;
use crate::models::job::Job;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, NodeTask, StoredNetEvent, StoredNodeEvent};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
// #[cfg(target_family = "unix")]
// use std::os::unix::fs::MetadataExt;
// #[cfg(target_family = "windows")]
// use std::os::windows::fs::MetadataExt;
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{
        mpsc::{self, Sender},
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
    // pub rx_recv: Arc<Receiver<NetResponse>>,
    _task: NodeTask,
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
        // let (tx_recv, _rx_recv) = mpsc::channel();

        /* 
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
                        ToNetwork::SendFile(file) => {
                            let data = std::fs::read(&file).unwrap();
                            let file_name = file.file_name().unwrap().to_str().unwrap();
                            let msg = NetMessage::SendFile(file_name.to_owned(), data);
                            // send file to all client
                            for peer in &peers {
                                handler.network().send(*peer, &msg.ser());
                            }
                        }
                        ToNetwork::Ping => {
                            // Should not be called from this level!
                        }
                        ToNetwork::Exit => {
                            // Wonder why I can't see this? Does it not stop?
                            println!("Terminate signal received!");
                            handler.stop();
                            break;
                        }
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
                                NetMessage::Ping(None) => {
                                    // we should not attempt to connect to the host!
                                    println!(
                                        "Received ping from client! Sending broadcast signal!"
                                    );
                                    let ping = NetResponse::Ping;
                                    tx_recv.send(ping).unwrap();
                                },
                                NetMessage::Ping(Some(_)) => {
                                    println!(
                                        "Received server ping, but we're the server? Ignoring"
                                    );
                                },
            
                                NetMessage::RequestJob => {
                                    todo!("Impl. Requesting job");
                                },
                                NetMessage::SendFile(file_name, data) => {
                                    let path = std::env::temp_dir().join(file_name);
                                    std::fs::write(&path, data).unwrap();
                                    tx_recv.send(NetResponse::FileTransfer(path)).unwrap();
                                },
                            }
                        }
                        StoredNetEvent::Connected(endpoint, _) => {
                            println!("Connected via TCP channel! [{}]", endpoint.addr());
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
        */

        Self {
            tx: Arc::new(tx),
            // rx_recv: Arc::new(rx_recv),
            _task: task,
            addr: public_addr,
        }
    }

    // TODO: repurpose this so that we're sending through NetworkService instead of interfacing to Server directly.
    #[allow(dead_code)]
    pub fn send_job(&self, _job: Job) {
        // if let Err(e) = self.tx.send(CmdMessage::SendJob(job)) {
        //     println!("Issue sending job request to server! {e}");
        // }
    }

    #[allow(dead_code)]
    pub fn connect_peer(&self, socket: SocketAddr) {
        self.tx.send(ToNetwork::Connect(socket)).unwrap();
    }
}

// impl NetworkNode for Server {
//     fn ping(&self) {
//         self.tx.send(ToNetwork::Ping).unwrap();
//     }

//     fn send_file(&self, file: PathBuf) {
//         if !&file.is_file() {
//             println!("file path is not a file! Aborting operation!");
//             return;
//         }
        
//         if let Err(e) = &self.tx.send(ToNetwork::SendFile(file))
//         {
//             println!("Failed to send file request to server! {e}");
//         }
//     }
// }

impl AsRef<SocketAddr> for Server {
    fn as_ref(&self) -> &SocketAddr {
        &self.addr
    }
}
