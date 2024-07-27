use crate::models::{
    job::Job,
    message::{Message, Signal},
    node::Node,
    project_file::ProjectFile,
    render_info::RenderInfo,
    render_queue::RenderQueue,
    server_setting::ServerSetting,
};
use anyhow::Result;
use blender::{manager::Manager as BlenderManager, models::mode::Mode};
use local_ip_address::local_ip;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use semver::Version;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use super::network::Network;

pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";

/*
    Let me design this real quick here - I need to setup a way so that once the server is running, it sends out a ping signal to notify any and all inactive client node on the network.
    Once the node receives the signal, it should try to re-connect to the server over TCP channel instead of UDP channel.

    server:udp -> ping { server ip's address } -> client:udp
    // currently client node is able to receive the server ping, but unable to connect to the server!
    client:tcp -> connect ( server ip's address ) -> ??? Err?
*/

// wish there was some ways to share Server and Client structs?
pub struct Server {
    handler: NodeHandler<Signal>,
    listeners: Option<NodeListener<Signal>>,
    nodes: Vec<Node>,
    public_addr: SocketAddr,
    pub job: Option<Job>,
}

impl Network for Server {
    /// send network message to specific endpoint
    fn send_to_target(&self, target: Endpoint, message: &Message) {
        self.handler.network().send(target, &message.ser());
    }

    /// Send message to all clients that's connected to the host.
    fn send_to_all(&self, message: &Message) {
        println!("Sending {:?} to the following client", &message);
        let data = message.ser();
        for node in &self.nodes {
            let status = self.handler.network().send(node.endpoint, &data);
            println!("{:?} to {} [{}]", status, node.name, node.endpoint.addr());
        }
        println!("End sending data");
    }
}

// Should I have a job manager here? Or should that be in it's own separate struct?
impl Server {
    pub fn new(port: u16) -> Result<Server> {
        let (handler, listeners) = node::split();

        let ip = match local_ip() {
            Ok(addr) => addr,
            Err(e) => {
                println!("Are you connected to the internet? Please check your network configuration! using localhost\n{:?}", e);
                IpAddr::V4(Ipv4Addr::LOCALHOST)
            }
        };
        let public_addr = SocketAddr::new(ip, port);

        // could this be the answer? Do I need to use the server's actual ip address instead? Could I not rely on the localhost ip?
        // let listen_addr = format!("127.0.0.1:{}", port);
        // unfortunately I would have to test this when I get back home. Currently, I cannot test this in the public network domain. I may ping other unwanted devices on the network.
        if let Err(e) = handler
            .network()
            .listen(Transport::FramedTcp, public_addr.to_string())
        {
            return Err(anyhow::anyhow!(
                "Failed to listen on port {} | \nErr: {:?}",
                port,
                e
            ));
        }
        handler.network().listen(Transport::Udp, MULTICAST_ADDR)?;

        Ok(Self {
            handler,
            listeners: Some(listeners),
            nodes: Vec::new(),
            public_addr,
            job: None,
        })
    }

    // Server listens
    pub fn run(&mut self) {
        let listener = self.listeners.take().unwrap();
        listener.for_each(move |event| match event {
            // interface from the network status
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(endpoint, established) => self.handle_connected(endpoint, established),
                NetEvent::Accepted(endpoint, _) => self.handle_accepted(endpoint),
                NetEvent::Message(endpoint, bytes) => self.handle_message(endpoint, bytes),
                NetEvent::Disconnected(endpoint) => self.handle_disconnected(endpoint),
            },
            // interface self generated by the nodes - Accept incoming command
            NodeEvent::Signal(signal) //=> match signal {
                // Signal::SendChunk => self.send_chunk(),
                => println!("Signal received, but not implemented! {:?}", signal),
                //},
            });
    }

    /// Once the server connects to node? Maybe this will never get called?
    fn handle_connected(&mut self, endpoint: Endpoint, established: bool) {
        // Did I accidentially multi-cast myself?
        // this is getting invoked by a multicast address, but I am not sure why?
        println!("A entity connected to the server! [{}]", endpoint.addr());
    }

    /// Server accepts connection if through TCP, UDP will always accept connection no matter what
    fn handle_accepted(&mut self, endpoint: Endpoint) {
        println!("Server acccepts connection: [{}]", endpoint.addr());
    }

    /// Receive message from client nodes
    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        // I wouldn't imagine having broken/defragmented packets within local network?
        match Message::de(bytes) {
            Ok(msg) => match msg {
                // a new node register itself to the network!
                Message::RegisterNode { name } => self.register_node(&name, endpoint),
                Message::UnregisterNode => self.unregister_node(endpoint),
                // Client should not be sending us the jobs!
                //Message::LoadJob() => {}
                Message::JobResult(render_info) => self.handle_job_result(endpoint, render_info),
                Message::CheckForBlender { os, version, arch } => {
                    self.check_for_blender(endpoint, &os, &arch, &version)
                }
                Message::ClientPing { name } => self.handle_client_ping(endpoint, &name),
                Message::ServerPing { socket } => {
                    println!(
                        "Server received server ping! but server do not handle such conditions! [{}]",
                        socket
                    )
                }
                // Message::LoadJob(_) => todo!(),
                // Message::CancelJob => todo!(),
                // Message::HaveMatchingBlenderRequirement { have_blender } => todo!(),
                // Message::FileRequest(_) => todo!(),
                // Message::Chunk(_) => todo!(),
                // Message::CanReceive(_) => todo!(),
                _ => println!("Unhandled case for {:?}", msg),
            },
            Err(_) => todo!(),
        }
    }

    /// A node has been disconnected from the network
    fn handle_disconnected(&mut self, endpoint: Endpoint) {
        println!("Disconnected event receieved! [{}]", endpoint.addr());
        // I believe there's a reason why I cannot use endpoint.addr()
        // Instead, I need to match endpoint to endpoint from node struct instead
        match self
            .nodes
            .iter()
            .position(|n| n.endpoint.addr() == endpoint.addr())
        {
            Some(index) => {
                let unit = self.nodes.remove(index);
                println!(
                    "Unregistered node '{}' with ip {}",
                    unit.name,
                    unit.endpoint.addr()
                );
            }
            None => {
                dbg!(&self.nodes, endpoint);
                panic!("This should never happen! Unless I got the address wrong again?");
            }
        }
    }

    /// If a client send out signal - we should try to establish connection.
    fn handle_client_ping(&mut self, endpoint: Endpoint, name: &str) {
        // we should not attempt to connect to the host!
        println!("Received ping from client '{}' [{}]", name, endpoint.addr());
        // self.register_node(&name, endpoint);
        let msg = Message::ServerPing {
            socket: self.public_addr,
        };
        self.send_to_target(endpoint, &msg);
    }

    /// Ping any inactive node to reconnect
    pub fn ping(&self) {
        match self
            .handler
            .network()
            .connect(Transport::Udp, MULTICAST_ADDR)
        {
            Ok((endpoint, _)) => {
                // create a server ping
                println!("Pinging inactive clients!");
                self.send_to_target(
                    endpoint,
                    &Message::ServerPing {
                        socket: self.public_addr,
                    },
                );
            }
            Err(e) => {
                eprintln!(
                    "Unable to send out ping signal! Check your internet configuration? {}",
                    e
                );
                return;
            }
        }
    }

    fn test_send_job_to_target_node(&mut self) {
        let blend_scene = PathBuf::from("./test.blend");
        let project_file = ProjectFile::new(blend_scene);
        let version = Version::new(4, 1, 0);
        let mode = Mode::Animation { start: 0, end: 2 };
        let server_config = ServerSetting::load();
        let job = Job::new(project_file, server_config.render_dir, version, mode);
        self.start_new_job(Some(job));
    }

    fn start_new_job(&mut self, new_job: Option<Job>) {
        if self.job.is_some() {
            println!("Uh oh there's previous job running at the moment!");
            // TODO handle conditions on the ongoing active job. Let the rendering node aware of the new job poll.
            let msg = Message::CancelJob;
            self.send_to_all(&msg);
        }

        self.job = new_job;

        let j: &mut Job = self.job.as_mut().unwrap();
        if let Some(frame) = &j.next_frame() {
            let queue = RenderQueue::new(*frame, j.version.clone(), j.project_file.clone(), j.id);
            let msg = Message::LoadJob(queue);
            self.send_to_all(&msg);
        }
    }

    /// Notify all clients a node has been registered (Connected)
    fn register_node(&mut self, name: &str, endpoint: Endpoint) {
        let local_ip = local_ip().unwrap();
        if endpoint.addr().ip() == local_ip {
            // in this case, we're connected to the server locally?
            // endpoint.set_addr(SocketAddr::new(local_ip, endpoint.addr().port()));
            println!("A client on this localhost is trying to connect to the server!",);
            endpoint.addr().set_ip(IpAddr::V4(Ipv4Addr::LOCALHOST));
        }

        dbg!(&endpoint);
        let node = Node::new(name, endpoint);

        println!(
            "Node Registered successfully! '{}' [{}]",
            name,
            &node.endpoint.addr()
        );

        self.nodes.push(node);

        // for testing purposes -
        // once we received a connection, we should give the node a new job if there's one available, or currently pending.
        // in this example here, we'll focus on sending a job to the connected node.
        self.test_send_job_to_target_node();
    }

    /// received notification from node being disconnected from the server.
    fn unregister_node(&mut self, endpoint: Endpoint) {
        match self
            .nodes
            .iter()
            .position(|n| n.endpoint.addr() == endpoint.addr())
        {
            Some(index) => {
                self.nodes.remove(index);
            }
            None => {
                println!("Foreign/Rogue node received! {}", endpoint.addr());
            }
        }
    }

    fn handle_job_result(&mut self, endpoint: Endpoint, render_info: RenderInfo) {
        println!("Job result received! {:?}", render_info);
        if let Some(job) = self.job.as_mut() {
            // TODO: Take a break and come back to this. try a different code block.
            job.renders.insert(render_info);
            match job.next_frame() {
                Some(frame) => {
                    let version = job.version.clone();
                    let project_file = job.project_file.clone();
                    let render_queue = RenderQueue::new(frame, version, project_file, job.id);
                    let message = Message::LoadJob(render_queue);
                    self.send_to_target(endpoint, &message);
                }
                None => {
                    // Job completed!
                    println!("Job completed!");
                    self.job = None; // eventually we will probably want to change this and make this better?
                }
            }
        }
    }

    /// A client request if other client have identical blender version
    fn check_for_blender(&mut self, endpoint: Endpoint, os: &str, arch: &str, version: &Version) {
        let current_os = std::env::consts::OS;
        let current_arch = std::env::consts::ARCH;
        let default_msg = Message::HaveMatchingBlenderRequirement {
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
            (current_os, current_arch) if current_os.eq(os) & current_arch.eq(arch) => {
                let manager = BlenderManager::load();
                let blender = manager.have_blender(&version);
                let msg = Message::HaveMatchingBlenderRequirement {
                    have_blender: blender,
                };
                self.send_to_target(endpoint, &msg);
            }
            (os, _) => {
                println!(
                    "Client [{}] have incompatible Arch! (Client[{}] != Target[{}])! Ignoring!",
                    endpoint.addr(),
                    arch,
                    current_arch
                );
                self.send_to_target(endpoint, &default_msg);
                return;
            }
            (_, _) => {
                println!(
                    "Client [{}] have incompatible OS! (Client[{}] != Target[{}])! Ignoring!",
                    endpoint.addr(),
                    os,
                    current_os
                );
                self.send_to_target(endpoint, &default_msg);
                return;
            }
        }

        if std::env::consts::OS != os {
            println!(
                "Client [{}] have incompatible OS (Client[{}] != Target[{}])! Ignoring!",
                endpoint.addr(),
                os,
                std::env::consts::OS
            );
            return;
        }

        // in this case, the client is asking all other client if any one have the matching blender version type.
        // let _map = self
        //     .nodes
        //     .iter()
        //     .filter(|n| n.endpoint != endpoint)
        //     .map(|n| self.send_to_target(n.endpoint, &msg));
    }
}
