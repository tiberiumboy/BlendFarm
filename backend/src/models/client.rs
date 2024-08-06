/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/
use super::{job::Job, message::CmdMessage, node::Node, server::MULTICAST_ADDR};
use crate::models::message::NetMessage;
use blender::blender;
use gethostname::gethostname;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, StoredNetEvent, StoredNodeEvent};
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};
use std::{collections::HashSet, net::SocketAddr, sync::mpsc, thread, time::Duration};

const INTERVAL_MS: u64 = 500;

pub struct Client {
    tx: mpsc::Sender<CmdMessage>,
    // name: String,
    // public_addr: SocketAddr, // I'm    not sure what this one is used for?
    // let's focus on getting the client connected for now.
    // Is there a way for me to hold struct objects while performing a transfer task?
    // ^Yes, box heap the struct! See example - https://github.com/sigmaSd/LanChat/blob/master/net/src/lib.rs
}

// I wonder if it's possible to combine server/client code together to form some kind of intristic networking solution?
impl Client {
    fn generate_ping(socket: &SocketAddr) -> NetMessage {
        NetMessage::Ping {
            name: gethostname().into_string().unwrap(),
            socket: socket.to_owned(),
            is_client: true,
        }
    }

    pub fn new() -> Client {
        let (handler, listener) = node::split::<NetMessage>();
        let public_addr = SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), 0);
        let (_task, mut receiver) = listener.enqueue();

        // listen tcp
        if let Err(e) = handler.network().listen(Transport::FramedTcp, public_addr) {
            panic!("Unable to listen to tcp! \n{}", e);
        };

        // listen udp
        if let Err(e) = handler.network().listen(Transport::Udp, MULTICAST_ADDR) {
            println!("Unable to listen to udp! \n{}", e);
        }

        // connect udp
        let udp_conn = match handler.network().connect(Transport::Udp, MULTICAST_ADDR) {
            Ok(conn) => conn,
            Err(e) => panic!("Somethiing terrible happen! {e:?}"),
        };

        let (tx, rx) = mpsc::channel();
        // let (tx_recv, rx_recv) = mpsc::channel::<RequestMessage>();

        thread::spawn(move || {
            let mut peers: HashSet<Node> = HashSet::new();
            let mut server: Option<Endpoint> = None;
            let mut _current_job: Option<Job> = None;

            // this will help contain the job list I need info on.
            // Feature: would this be nice to load any previous known job list prior to running this client?
            // E.g. store in json file if client gets shutdown
            // let mut jobs: HashSet<Job> = HashSet::new();

            loop {
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        CmdMessage::AddPeer { name, socket } => {
                            let (peer_endpoint, _) = handler
                                .network()
                                .connect(Transport::FramedTcp, socket)
                                .unwrap();
                            let node = Node::new(&name, peer_endpoint);
                            peers.insert(node);
                        }
                        CmdMessage::SendJob(job) => {
                            // TODO: Find a way to set a new job here and begin forth?
                            dbg!(job);
                            // jobs.insert(job);
                        }
                        CmdMessage::Ping => {
                            // send a ping to the network
                            println!("Received a ping request!");
                            handler
                                .network()
                                .send(udp_conn.0, &Self::generate_ping(&public_addr).ser());
                        }
                        CmdMessage::AskForBlender { version } => {
                            if let Some(conn) = server {
                                let msg = NetMessage::CheckForBlender {
                                    os: std::env::consts::OS.to_owned(),
                                    version,
                                    arch: std::env::consts::ARCH.to_owned(),
                                    caller: public_addr,
                                };
                                handler.network().send(conn, &msg.ser());
                            }
                        }
                        CmdMessage::Exit => {
                            // Terminated signal received!
                            // should we also close the receiver?
                            handler.stop();
                            break;
                        } // CmdMessage::Render => {
                          //     // Begin the render process!
                          //     if let Some(ref job) = current_job {
                          //         let mut manager = blender::Manager::load();
                          //         // eventually I will need to find a way to change this so that I could use the network to ask other client for version of blender.
                          //         // if no other client are available then download blender from the web.
                          //         // let blender = manager.get_blender(&job.version).unwrap();
                          //     }
                          // }
                    }
                }

                if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                    match event {
                        StoredNetEvent::Connected(endpoint, _) => {
                            // we connected via udp channel!
                            if endpoint == udp_conn.0 {
                                println!("Connected via UDP channel! [{}]", endpoint.addr());

                                // we then send out a ping signal on udp channel
                                handler
                                    .network()
                                    .send(udp_conn.0, &Self::generate_ping(&public_addr).ser());
                            }
                            // we connected via tcp channel!
                            else {
                                println!("Connected via TCP channel! [{}]", endpoint.addr());
                                server = Some(endpoint);

                                // sending job request
                                handler
                                    .network()
                                    .send(endpoint, &NetMessage::RequestJob.ser());
                                // dbg!(handler.network().send(endpoint, ))
                            }
                        }
                        StoredNetEvent::Accepted(endpoint, _) => {
                            // an tcp connection accepts the connection!
                            println!("Client was accepted to server! [{}]", endpoint.addr());
                            // self.server_endpoint = Some(endpoint);
                        }
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let message = match NetMessage::de(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    println!("unable to deserialize net message! \n{}", e);
                                    continue;
                                }
                            };

                            println!(
                                "Message received from [{}] \n{:?}!",
                                endpoint.addr(),
                                &message
                            );

                            match message {
                                // server to client
                                // we received a ping signal from the server that accepted our ping signal.
                                // this means that either the server send out a broadcast signal to identify lost node connections on the network
                                NetMessage::Ping {
                                    name,
                                    socket,
                                    is_client: false,
                                } => {
                                    println!(
                                        "Hey! Client received a multicast ping signal from Server [{}]!",
                                        &socket
                                    );

                                    if server.is_some() {
                                        println!("This node is already connected to the server! Ignoring!");
                                        continue;
                                    }

                                    match handler.network().connect(Transport::FramedTcp, socket) {
                                        Ok((endpoint, _)) => {
                                            peers.insert(Node::new(&name, endpoint));
                                        }
                                        Err(e) => {
                                            println!("Error connecting to the server! \n{}", e);
                                        }
                                    }
                                }
                                NetMessage::Ping {
                                    is_client: true, ..
                                } => {
                                    // ignore the ping signal from the client
                                }
                                NetMessage::SendJob(job) => {
                                    println!("Received a new job!\n{:?}", job);
                                    _current_job = Some(job);

                                    // First let's check if we have the correct blender installation
                                    // then check and see if we have the files?
                                    // if !.project_file.file_path().exists() {
                                    //     // here we will fetch the file path from the server
                                    //     // but for now let's continue.
                                    //     println!("Path does not exist!");
                                    // }
                                    /*

                                    // run the blender() - this will take some time. Could implement async/thread?
                                    match render_queue.run(1) {
                                        // returns frame and image path
                                        Ok(render_info) => {
                                            println!(
                                                "Render completed! Sending image to server! {:?}",
                                                render_info
                                            );

                                            let mut file_transfer = FileTransfer::new(
                                                render_info.path.clone(),
                                                endpoint,
                                            );

                                            // yeah gonna have to revisit this part...
                                            // file_transfer.transfer(&handler);
                                            // is there a way to convert mutable to immutable?

                                            // self.file_transfer = Some(file_transfer);
                                            // wonder if there's a way to say - hey I've completed my transfer,
                                            // please go and look in your download folder with this exact file name,
                                            // then proceed to your job manager to move out to output destination.
                                            // first notify the server that the job is completed and prepare to receive the file
                                            let msg = NetMessage::JobResult(render_info);
                                            handler.network().send(endpoint, &msg.ser());

                                            // let msg = Message::FileRequest(info);
                                            // self.send_to_target(self.server_endpoint, msg);
                                        }
                                        Err(e) => println!("Fail to render on client! {:?}", e),
                                    }
                                    */
                                }
                                NetMessage::CheckForBlender {
                                    os,
                                    version,
                                    arch,
                                    caller,
                                } if os == std::env::consts::OS
                                    && arch == std::env::consts::ARCH =>
                                {
                                    println!("Received a blender check request from [{}]", caller);

                                    // here we will check and see if we have blender installed
                                    let manager = blender::Manager::load();
                                    if manager
                                        .get_blenders()
                                        .iter()
                                        .any(|b| b.get_version().eq(&version))
                                    {
                                        // send a reply back!
                                        match handler
                                            .network()
                                            .connect(Transport::FramedTcp, caller)
                                        {
                                            Ok((endpoint, _)) => {
                                                handler.network().send(
                                                    endpoint,
                                                    &NetMessage::CanReceive(true).ser(),
                                                );
                                            }
                                            Err(e) => {
                                                println!("Error connecting to the client! \n{}", e);
                                            }
                                        }
                                    }
                                    // let have_blender = check_blender(&os, &version, &arch);
                                    // self.contain_blender_response(endpoint, have_blender);
                                }
                                // client to client
                                _ => {
                                    println!(
                                        "Unhandled client message case condition for {:?}",
                                        message
                                    );
                                }
                            };
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            // TODO: How can we initialize another listening job? We definitely don't want the user to go through the trouble of figuring out which machine has stopped.
                            // Disconnected was call when server was shut down
                            println!("Lost connection to host! [{}]", endpoint.addr());
                            server = None;
                            // in the case of this node disconnecting, I would like to auto renew the connection if possible.
                        }
                    }
                }
            }
        });

        Self {
            tx,
            // rx_recv,
            // name: gethostname().into_string().unwrap(),
            // public_addr,
        }
    }

    // TODO: find a way to set up invoking mechanism to auto ping out if we do not have any connection to the server
    pub fn ping(&self) {
        println!("Sending ping command from client");
        self.tx.send(CmdMessage::Ping).unwrap();
    }

    pub fn ask_for_blender(&self, version: Version) {
        self.tx.send(CmdMessage::AskForBlender { version }).unwrap();
    }

    // same here...Why am I'm unsure of everything in my life?
    // Let's not worry about this for now...
    /*

    fn contain_blender_response(&self, endpoint: Endpoint, have_blender: bool) {
        if !have_blender {
            println!(
                "Client [{}] does not have the right version of blender installed",
                endpoint.addr()
            );
            return;
        }

        self.handler
            .network()
            .send(endpoint, &NetMessage::CanReceive(true).ser());
    }

    fn file_request(&mut self, endpoint: Endpoint, file_info: &FileInfo) {
        println!("name: {:?} | size: {}", file_info.path, file_info.size);
        let message = NetMessage::CanReceive(true);
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
        // TODO: Find a way to send file from one computer to another!
    }
    */
}

// TODO: I do need to implement a Drop method to handle threaded task. Making sure they're close is critical!
impl Drop for Client {
    fn drop(&mut self) {
        self.tx.send(CmdMessage::Exit).unwrap();
    }
}
