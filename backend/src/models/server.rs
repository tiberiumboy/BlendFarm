use super::message::CmdMessage;
use super::project_file::ProjectFile;
use super::server_setting::ServerSetting;
use crate::models::{job::Job, message::NetMessage, node::Node};
use anyhow::Result;
use blender::models::mode::Mode;
use gethostname::gethostname;
use local_ip_address::local_ip;
use message_io::network::Transport;
use message_io::node::{self, NodeTask, StoredNetEvent, StoredNodeEvent};
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::mpsc::{self, SendError};
use std::{collections::HashSet, net::SocketAddr, thread, time::Duration};

pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";
const INTERVAL_MS: u64 = 500;

/*
    Let me design this real quick here - I need to setup a way so that once the server is running, it sends out a ping signal to notify any and all inactive client node on the network.
    Once the node receives the signal, it should try to re-connect to the server over TCP channel instead of UDP channel.

    server:udp -> ping { server ip's address } -> client:udp
    // currently client node is able to receive the server ping, but unable to connect to the server!
    client:tcp -> connect ( server ip's address ) -> ??? Err?
*/

// wish there was some ways to share Server and Client structs?
// Issue: Cannot derive debug because NodeTask doesn't derive Debug! Omit NodeTask if you need to Debug!
pub struct Server {
    tx: mpsc::Sender<CmdMessage>,
    // rx_recv: mpsc::Receiver<RequestMessage>,
    // public_addr: SocketAddr,
    _task: NodeTask,
}

impl Server {
    fn generate_ping(socket: &SocketAddr) -> NetMessage {
        NetMessage::Ping {
            name: gethostname().into_string().unwrap(),
            socket: socket.to_owned(),
            is_client: false,
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
        let udp_conn = handler
            .network()
            .connect(Transport::Udp, MULTICAST_ADDR)
            .unwrap();

        // listen udp
        handler
            .network()
            .listen(Transport::Udp, MULTICAST_ADDR)
            .unwrap();

        // this is starting to feel like event base driven programming?
        // is this really the best way to handle network messaging?
        let (tx, rx) = mpsc::channel();

        // use tx_recv to receive signal commands from the network
        // let (tx_recv, rx_recv) = mpsc::channel();

        thread::spawn(move || {
            let mut peers: HashSet<Node> = HashSet::new();
            let mut current_job: Option<Job> = None;

            loop {
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        CmdMessage::SendJob(job) => {
                            // send new job to all clients
                            dbg!(&peers);
                            let info = &NetMessage::SendJob(job);
                            // send to all connected clients on udp channel
                            for peer in peers.iter() {
                                dbg!(peer.endpoint.addr());
                                handler.network().send(peer.endpoint, &info.ser());
                            }
                        }
                        CmdMessage::AddPeer { name, socket } => {
                            // hmm wonder what this'll do?
                            match handler.network().connect(Transport::FramedTcp, socket) {
                                Ok((peer, _)) => {
                                    println!("Connected to peer `{}` [{}]", name, peer.addr());
                                    peers.insert(Node::new(&name, peer));
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
                                .send(udp_conn.0, &Self::generate_ping(&public_addr).ser());
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
                                handler.network().send(peer.endpoint, &info.ser());
                            }
                        }
                        CmdMessage::Exit => {
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
                                // I'm working on something I don't know if it'll work or not...
                                // this I can omit, but how do I make this code better?
                                NetMessage::CheckForBlender { caller, .. } => {
                                    // omit the caller from the list of peers
                                    for peer in peers.iter().filter(|p| p.endpoint.addr() != caller)
                                    {
                                        handler.network().send(peer.endpoint, &msg.ser());
                                    }
                                }
                                NetMessage::Ping {
                                    name,
                                    is_client: true,
                                    ..
                                } => {
                                    // we should not attempt to connect to the host!
                                    println!("Received ping from client '{}'", name);

                                    // maybe I should just send out a server ping signal instead?
                                    handler
                                        .network()
                                        .send(udp_conn.0, &Self::generate_ping(&public_addr).ser());
                                }
                                NetMessage::Ping {
                                    is_client: false, ..
                                } => {
                                    // do nothing for now
                                }
                                NetMessage::SendJob(job) => {
                                    println!("Received job from [{}]\n{:?}", endpoint.addr(), job);
                                    // current_job = Some(job);
                                }
                                NetMessage::RequestJob => {
                                    // at this point here, client is asking us for a new job.
                                    if let Some(ref job) = current_job {
                                        let job = job.clone();
                                        handler
                                            .network()
                                            .send(endpoint, &NetMessage::SendJob(job).ser());
                                    } else {
                                        println!("No job available to send!");
                                    }
                                }
                                _ => println!("Unhandled case for {:?}", msg),
                            }
                        }
                        StoredNetEvent::Connected(endpoint, _) => {
                            // we connected via udp channel!
                            if endpoint == udp_conn.0 {
                                println!("Connected via UDP channel! [{}]", endpoint.addr());
                                // could I send them back to connect?
                                handler
                                    .network()
                                    .send(endpoint, &Self::generate_ping(&public_addr).ser());
                            }
                            // we connected via tcp channel!
                            else {
                                println!("Connected via TCP channel! [{}]", endpoint.addr());
                            }
                        }
                        // wonder what I can do with resource id?
                        StoredNetEvent::Accepted(endpoint, _) => {
                            println!("Server accepts connection: [{}]", endpoint);
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            println!("Disconnected event receieved! [{}]", endpoint.addr());
                            // I believe there's a reason why I cannot use endpoint.addr()
                            // Instead, I need to match endpoint to endpoint from node struct instead
                            let target = Node::new(&String::new(), endpoint);
                            if peers.remove(&target) {
                                println!("[{}] has disconnected", endpoint.addr());
                            }
                        }
                    }
                }
            }
        });

        Self {
            tx,
            // rx_recv,
            // public_addr,
            _task,
        }
    }

    pub fn send_job(&self, job: Job) -> Result<(), SendError<CmdMessage>> {
        self.tx.send(CmdMessage::SendJob(job))
    }

    pub fn test_send_job_to_target_node(&self) {
        let blend_scene = PathBuf::from("./test.blend");
        let project_file = ProjectFile::new(blend_scene);
        let version = Version::new(4, 1, 0);
        let mode = Mode::Animation { start: 0, end: 2 };
        let server_config = ServerSetting::load();
        let job = Job::new(project_file, server_config.render_dir, version, mode);

        // begin api invocation test
        match self.send_job(job) {
            Ok(_) => println!("Job sent successfully!"),
            Err(e) => println!("Error sending job! {:?}", e),
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

    pub fn ask_for_blender(&self, version: Version) {
        self.tx.send(CmdMessage::AskForBlender { version }).unwrap();
    }

    // going to have to put a hold on this for now...
    // code may not be useful anymore. may have to refactor this to make this function first.
    // Then we'll undo this.
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
