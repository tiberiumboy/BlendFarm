use crate::models::{
    job::Job, message::NetMessage, node::Node, project_file::ProjectFile,
    server_setting::ServerSetting,
};
use anyhow::Result;
use blender::models::mode::Mode;
use gethostname::gethostname;
use message_io::network::Transport;
use message_io::node::{self, StoredNetEvent, StoredNodeEvent};
use semver::Version;
use std::collections::HashSet;
use std::sync::mpsc::{self, SendError};
use std::time::Duration;
use std::{net::SocketAddr, path::PathBuf, thread};

use super::message::CmdMessage;

pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";

/*
    Let me design this real quick here - I need to setup a way so that once the server is running, it sends out a ping signal to notify any and all inactive client node on the network.
    Once the node receives the signal, it should try to re-connect to the server over TCP channel instead of UDP channel.

    server:udp -> ping { server ip's address } -> client:udp
    // currently client node is able to receive the server ping, but unable to connect to the server!
    client:tcp -> connect ( server ip's address ) -> ??? Err?
*/

// wish there was some ways to share Server and Client structs?
#[derive(Debug)]
pub struct Server {
    tx: mpsc::Sender<CmdMessage>,
    rx_recv: mpsc::Receiver<NetMessage>,
    public_addr: SocketAddr,
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
    pub fn new() -> Server {
        let (handler, listener) = node::split::<NetMessage>();

        let (_task, mut receiver) = listener.enqueue();

        // listen to tcp
        let (_, public_addr) = handler
            .network()
            .listen(Transport::FramedTcp, "0.0.0.0:0")
            .unwrap();

        // listen to udp
        handler
            .network()
            .listen(Transport::Udp, MULTICAST_ADDR)
            .unwrap();

        // connect to udp
        // what if this is the culprit?
        let udp_conn = handler
            .network()
            .connect(Transport::Udp, MULTICAST_ADDR)
            .unwrap();

        // this is starting to feel like event base driven programming?
        // is this really the best way to handle network messaging?
        let (tx, rx) = mpsc::channel();
        let (_tx_recv, rx_recv) = mpsc::channel();

        thread::spawn(move || {
            let mut peers: HashSet<Node> = HashSet::new();

            loop {
                std::thread::sleep(Duration::from_millis(500));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        CmdMessage::SendJob(job) => {
                            // send new job to all clients
                            let info = &NetMessage::SendJob(job).ser();
                            // send to all connected clients on udp channel
                            for peer in peers.iter() {
                                handler.network().send(peer.endpoint, &info);
                            }
                        }
                        CmdMessage::AddPeer { name, socket } => {
                            // hmm wonder what this'll do?
                            if let Ok((peer, _)) =
                                handler.network().connect(Transport::FramedTcp, socket)
                            {
                                peers.insert(Node::new(&name, peer));
                            }
                        }
                        CmdMessage::Ping => {
                            // send ping to all clients
                            handler
                                .network()
                                .send(udp_conn.0, &Self::generate_ping(&public_addr).ser());
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
                        StoredNetEvent::Message(_endpoint, bytes) => {
                            let msg = match NetMessage::de(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    println!("Error deserializing net message data! \n{e}");
                                    continue;
                                }
                            };

                            // I wouldn't imagine having broken/defragmented packets within local network?
                            match msg {
                                //         // a new node register itself to the network!
                                //         NetMessage::RegisterNode { name } => {
                                //             let local_ip = local_ip().unwrap();
                                //             if endpoint.addr().ip() == local_ip {
                                //                 // in this case, we're connected to the server locally?
                                //                 // endpoint.set_addr(SocketAddr::new(local_ip, endpoint.addr().port()));
                                //                 println!("A client on this localhost is trying to connect to the server!",);
                                //                 endpoint.addr().set_ip(IpAddr::V4(Ipv4Addr::LOCALHOST));
                                //             }

                                //             let node = Node::new(&name, endpoint);

                                //             println!(
                                //                 "Node Registered successfully! '{}' [{}]",
                                //                 &name,
                                //                 &endpoint.addr()
                                //             );

                                //             peers.insert(node);

                                //             // for testing purposes -
                                //             // once we received a connection, we should give the node a new job if there's one available, or currently pending.
                                //             // in this example here, we'll focus on sending a job to the connected node.
                                //             // self.test_send_job_to_target_node();
                                //         }
                                //         NetMessage::UnregisterNode => {}
                                //         // Client should not be sending us the jobs!
                                //         //Message::LoadJob() => {}
                                //         NetMessage::JobResult(render_info) => {
                                //             // println!("Job result received! {:?}", render_info);
                                //             // // yeahhhh about this... I need to do something about this..
                                //             // if let Some(job) = self.job.as_mut() {
                                //             //     // TODO: Take a break and come back to this. try a different code block.
                                //             //     job.renders.insert(render_info);
                                //             //     match job.next_frame() {
                                //             //         Some(frame) => {
                                //             //             let version = job.version.clone();
                                //             //             let project_file = job.project_file.clone();
                                //             //             let render_queue = RenderQueue::new(frame, version, project_file, job.id);
                                //             //             let message = Message::LoadJob(render_queue);
                                //             //             dbg!(handler.network().send(endpoint, &message.ser()));

                                //             //         }
                                //             //         None => {
                                //             //             // Job completed!
                                //             //             println!("Job completed!");
                                //             //             self.job = None; // eventually we will probably want to change this and make this better?
                                //             //         }
                                //             //     }
                                //             // }
                                //         }
                                //         NetMessage::CheckForBlender { os, version, arch } => {
                                //             let msg =
                                //                 Self::check_for_blender(endpoint, &os, &arch, &version);
                                //             handler.network().send(endpoint, &msg.ser());
                                //         }
                                NetMessage::Ping {
                                    name,
                                    socket,
                                    is_client: true,
                                } => {
                                    // we should not attempt to connect to the host!
                                    println!("Received ping from client '{}' [{}]", name, socket);

                                    // maybe I should just send out a server ping signal instead?
                                    handler
                                        .network()
                                        .send(udp_conn.0, &Self::generate_ping(&socket).ser());

                                    // so what happen here?
                                    // if let Ok((peer, _)) =
                                    //     handler.network().connect(Transport::FramedTcp, socket)
                                    // {
                                    //     println!("Connected to peer! [{}]", peer.addr());
                                    //     peers.insert(Node::new(&name, peer));
                                    // } else {
                                    //     println!("Unable to connect to {}", &socket);
                                    // }
                                }
                                NetMessage::Ping {
                                    name,
                                    socket,
                                    is_client: false,
                                } => {
                                    println!("Received server ping! {}, {}", socket, name);
                                }
                                //         // Message::LoadJob(_) => todo!(),
                                //         // Message::CancelJob => todo!(),
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
                        StoredNetEvent::Accepted(endpoint, _) => {
                            println!("Server acccepts connection: [{}]", endpoint.addr());
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            println!("Disconnected event receieved! [{}]", endpoint.addr());
                            // I believe there's a reason why I cannot use endpoint.addr()
                            // Instead, I need to match endpoint to endpoint from node struct instead
                            let target = Node::new(&String::new(), endpoint);
                            if peers.remove(&target) {
                                println!("[{}] has disconnected", target.endpoint.addr());
                            }
                        }
                    }
                }
            }
        });

        Self {
            tx,
            rx_recv,
            public_addr,
        }
    }

    pub fn send_job(&self, job: Job) -> Result<(), SendError<CmdMessage>> {
        self.tx.send(CmdMessage::SendJob(job))
    }

    fn test_send_job_to_target_node(&self) {
        let blend_scene = PathBuf::from("./test.blend");
        let project_file = ProjectFile::new(blend_scene);
        let version = Version::new(4, 1, 0);
        let mode = Mode::Animation { start: 0, end: 2 };
        let server_config = ServerSetting::load();
        let job = Job::new(project_file, server_config.render_dir, version, mode);

        // begin api invocation test
        self.send_job(job);
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
