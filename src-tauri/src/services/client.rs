/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/
use super::message::{NetResponse, ToNetwork};
use super::network_service::NetworkNode;
use crate::models::job::Job;
use crate::services::message::NetMessage;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, StoredNetEvent, StoredNodeEvent};
use std::fs::{self, File};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{net::SocketAddr, sync::mpsc, thread, time::Duration};

const INTERVAL_MS: u64 = 50;

pub struct Client {
    host: Arc<RwLock<Option<Endpoint>>>,
    tx: mpsc::Sender<ToNetwork>,
    addr: SocketAddr,
 }

// I wonder if it's possible to combine server/client code together to form some kind of intristic networking solution?

impl Client {
    pub fn new() -> Self {
        let (handler, listener) = node::split::<NetMessage>();
        let (_task, mut receiver) = listener.enqueue();

        let public_addr = SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), 0);

        // listen tcp
        if let Err(e) = handler.network().listen(Transport::FramedTcp, public_addr) {
            panic!("Unable to listen to tcp! \n{}", e);
        };

        let (tx, rx) = mpsc::channel();
        let (tx_recv, _rx_recv) = mpsc::channel();
        let host:Arc<RwLock<Option<Endpoint>>> = Arc::new(RwLock::new(None));
        let server = host.clone();

        thread::spawn(move || {
            // this will help contain the job list I need info on.
            // Feature: would this be nice to load any previous known job list prior to running this client?
            // E.g. store in json file if client gets shutdown
            let mut _current_job: Option<Job> = None;

            loop {
                // TODO: Find a better way to handle this instead of sleeping every few seconds.
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));

                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        ToNetwork::Connect(addr) => {
                            handler
                                .network()
                                .connect(Transport::FramedTcp, addr)
                                .unwrap();
                        }
                        ToNetwork::SendFile(file) => {
                            let server = server.read().unwrap();
                            if let Some(host) = *server {
                                let data = std::fs::read(&file).unwrap();
                                let file_name = file.file_name().unwrap().to_str().unwrap();
                                let msg = NetMessage::SendFile(file_name.to_owned(), data);
                                handler.network().send(host, &msg.ser());
                            }
                        }
                        ToNetwork::Ping => {
                            // send a ping to the network
                            // println!("Received a ping request!");
                            // handler.network().send(
                            //     udp_conn,   // why am I'm sending ping signal over udp connection? I don't have it anymore?
                            //     &NetMessage::Ping(None).serialize(),
                            // );

                            // this function should have been called within Network service only.
                            panic!("Should not happen...");
                        }
                        ToNetwork::Exit => {
                            // Terminated signal received!
                            // should we also close the receiver?
                            handler.stop();
                            break;
                        }
                    }
                }

                if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                    match event {
                        StoredNetEvent::Connected(endpoint, _) => {
                            // // we connected via udp channel!
                            // if endpoint == udp_conn {
                            //     println!("Connected via UDP channel! [{}]", endpoint.addr());

                            //     // we then send out a ping signal on udp channel
                                
                            // }
                            // we connected via tcp channel!
                            // else {
                                println!("Connected via TCP channel! [{}]", endpoint.addr());
                                let mut server = server.write().unwrap();
                                *server = Some(endpoint);
                                // TODO: should this be a default thing once client node is connected?
                                handler
                                    .network()
                                    .send(endpoint, &NetMessage::RequestJob.ser());
                            // }
                        }
                        StoredNetEvent::Accepted(endpoint, _) => {
                            // an tcp connection accepts the connection!
                            println!("Client was accepted to server! [{}]", endpoint.addr());
                            let mut server = server.write().unwrap();
                            // Ask Bogdan how I can update this value at runtime?
                            *server = Some(endpoint);
                        }
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let message = match NetMessage::de(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    println!("unable to deserialize net message! \n{}", e);
                                    continue;
                                }
                            };

                            println!("Message received from [{}]!", endpoint.addr());

                            match message {
                                // server to client
                                // we received a ping signal from the server that accepted our ping signal.
                                // this means that either the server send out a broadcast signal to identify lost node connections on the network
                                NetMessage::Ping (Some(socket)) => {
                                    let server = server.write().unwrap();
                                    if server.is_none() { 
                                        handler.network().connect(Transport::FramedTcp, socket).unwrap();
                                    }
                                }
                                // client to client - we simply ignore them.
                                NetMessage::Ping(_) => { },
                                NetMessage::SendFile(file_name, data) => {
                                    // Problem - program crash if the file already exist -
                                    // need to save the file in temp location first, then move into the directory when completed.
                                    // if duplicated file exist - find the best mitigate plan? e.g. metadata comparison

                                    // if let Err(e) = std::fs::copy(&src, &dst) {
                                    //     // this may happen when the user moves computer (MacOS Transfer) - Temp storage do not exist because it has been renamed to something else.
                                    //     // TODO: validate temp storage exist before using it. - ServerSetting::load()
                                    //     println!("Unable to copy file from [{src:?}] to [{dst:?}]: {e}");
                                    // }

                                    let tmp = std::env::temp_dir().join(&file_name);

                                    if tmp.exists() {
                                        if let Err(e) = fs::remove_file(&tmp) {
                                            println!("Unable to delete existing file in tmp?\n{tmp:?}\n{e}");
                                            return;
                                        }
                                    }

                                    let mut file = File::create_new(&tmp).unwrap();
                                    match file.write_all(&data) {
                                        Ok(_) => tx_recv.send(NetResponse::FileTransfer(tmp)).unwrap(),
                                        Err(e) => println!("Fail to create new temp file! \n{e}")  
                                    };

                                    // need to relocate this somewhere else?
                                    // let server = ServerSetting::load();
                                    // let output = server.blend_dir.join(file_name);
                                    // if output.exists() {
                                    //     if let Err(e) = fs::remove_file(&output) {
                                    //         println!("Problem removing file!\n{e}");
                                    //     }
                                    // }

                                    // // if the file doesn't exist then we can simply move it to output path.
                                    // if let Err(e) = fs::rename(&tmp, &output) {
                                    //     println!("Fail to move the file from temp [{tmp:?}] to destination [{output:?}]\n{e}");
                                    // }

                                    // println!(
                                    //     "Successfully received a file from {}",
                                    //     endpoint.addr()
                                    // );
                                },
                                // Strange - we shouldn't have to ask another client for job? This is meant for the server only.
                                NetMessage::RequestJob => {}
                            };
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            // TODO: How can we initialize another listening job? We definitely don't want the user to go through the trouble of figuring out which machine has stopped.
                            // Disconnected was call when server was shut down
                            println!("Lost connection to host! [{}]", endpoint.addr());
                            let mut server = server.write().unwrap();
                            *server = None;
                            tx_recv.send(NetResponse::Disconnected { socket: endpoint.addr() }).unwrap();
                        }
                    }
                }
            }
        });

        Self {
            host,
            tx,
            addr: public_addr,
        }
    }

    pub fn connect(&mut self, addr: SocketAddr) {
        if self.host.read().unwrap().is_none() {
            println!("Will try to connect!");
            self.tx.send(ToNetwork::Connect(addr)).unwrap();
        }
    }

    pub fn is_connected(&self) -> bool {
        self.host.read().unwrap().is_some()
    }
}


impl NetworkNode for Client {
    fn ping(&self) {
        self.tx.send(ToNetwork::Ping).unwrap();
    }

    fn send_file(&self, file: PathBuf) {
        self.tx.send(ToNetwork::SendFile(file)).unwrap();
    }
}

impl AsRef<SocketAddr> for Client {
    fn as_ref(&self) -> &SocketAddr {
        &self.addr
    }
}

// TODO: I do need to implement a Drop method to handle threaded task. Making sure they're close is critical!
impl Drop for Client {
    fn drop(&mut self) {
        self.tx.send(ToNetwork::Exit).unwrap();
    }
}
