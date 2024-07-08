use crate::models::{job::Job, message::Message, node::Node};
use anyhow::Result;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::net::SocketAddr;

use super::message::Signal;
use super::render_info::RenderInfo;
use super::render_queue::RenderQueue;

pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";

pub struct Server {
    handler: NodeHandler<Signal>,
    listeners: Option<NodeListener<Signal>>,
    nodes: Vec<Node>,
    job: Option<Job>,
    public_addr: SocketAddr,
}

// Should I have a job manager here? Or should that be in it's own separate struct?

impl Server {
    pub fn new(port: u16) -> Result<Server> {
        let (mut handler, listeners) = node::split();

        let ip = local_ip().unwrap();
        let public_addr = SocketAddr::new(ip, port);

        Self::ping(&public_addr, &mut handler); // ping the inactive clients, if there are any

        let listen_addr = format!("127.0.0.1:{}", port);
        handler
            .network()
            .listen(Transport::FramedTcp, listen_addr)?;
        handler.network().listen(Transport::Udp, MULTICAST_ADDR)?;

        Ok(Self {
            handler,
            listeners: Some(listeners),
            nodes: Vec::new(),
            job: None,
            public_addr,
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
        // todo!("Figure out how this is invoked, and then update the implmentation below.");
        // this is getting invoked by a multicast address, but I am not sure why?
        println!(
            "Something connected to the server! {}, {}",
            endpoint, established
        );
    }

    /// Server accepts connection if through TCP, UDP will always accept connection no matter what
    fn handle_accepted(&mut self, endpoint: Endpoint) {
        println!("Server acccepts connection: [{}]", endpoint.addr());
    }

    /// Receive message from client nodes
    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        // problem here? I am trying to deserialize Job, but unwrap reports io error: Unexpected end of file message! Figure out why! String works?
        let msg: Message = bincode::deserialize(bytes).unwrap();
        match msg {
            // a new node register itself to the network!
            Message::RegisterNode { name, addr } => self.register_node(&name, addr, endpoint),
            Message::UnregisterNode { addr } => self.unregister_node(addr),
            // Client should not be sending us the jobs!
            //Message::LoadJob() => {}
            Message::JobResult(render_info) => self.handle_job_result(endpoint, render_info),
            Message::HaveBlender { .. } => self.ask_client_for_blender(endpoint, &msg),

            // confirmed to recived, but do absolutely nothing! Server shall not care!
            Message::Ping { addr, host } => {
                if host {
                    println!("Server pinged itself! {:?}", addr);
                } else {
                    println!("Server pinged by {:?}", addr);
                    let msg = Message::Ping {
                        addr: self.public_addr,
                        host: true,
                    };
                    self.send_to_target(endpoint, &msg);
                }
                // should a node be able to reply back to this?
                // we do not care - we skip this. Log perhaps?
            }
            _ => println!("Unhandled case for {:?}", msg),
        }
    }

    /// A node has been disconnected from the network
    fn handle_disconnected(&mut self, endpoint: Endpoint) {
        // I believe there's a reason why I cannot use endpoint.addr()
        // Instead, I need to match endpoint to endpoint from node struct instead
        match self.nodes.iter().position(|n| n.endpoint == endpoint) {
            Some(index) => {
                let unit = self.nodes.remove(index);
                let msg = Message::UnregisterNode { addr: unit.addr };
                self.send_to_all(&msg);
                println!("Unregistered node '{}' with ip {}", unit.name, unit.addr);
            }
            None => {
                panic!("This should never happen! Unless I got the address wrong again?");
            }
        }
    }

    /// Ping any inactive node to reconnect
    pub fn ping(addr: &SocketAddr, handler: &mut NodeHandler<Signal>) {
        // attempt to connect to multicast address
        // maybe this is the problem?
        let (endpoint, _) = handler
            .network()
            .connect(Transport::Udp, MULTICAST_ADDR)
            .unwrap();

        // create a server ping
        // I feel like this is such a dangerous power move here?
        let msg = Message::Ping {
            addr: *addr,
            host: true,
        };
        println!("Pinging inactive clients! {:?}", &msg);

        let data = bincode::serialize(&msg).unwrap();
        handler.network().send(endpoint, &data);
    }

    /// Notify all clients a node has been registered (Connected)
    fn register_node(&mut self, name: &str, addr: SocketAddr, endpoint: Endpoint) {
        let node = Node::new(name, addr, endpoint);
        self.send_to_all(&Message::RegisterNode {
            name: node.name.clone(),
            addr: node.addr,
        });

        // here we can invoke new container to hold the incoming connection.
        self.send_to_target(endpoint, &self.create_node_list());

        // for testing purposes -
        // let's start working from here
        // once we received a connection, we should give the node a new job if there's one available, or currently pending.
        // in this example here, we'll focus on sending a job to the connected node.
        // self.send_job_to_node(&node);

        self.nodes.push(node);

        println!("Node Registered successfully! '{}' [{}]", name, addr);
    }

    /// received notification from node being disconnected from the server.
    fn unregister_node(&mut self, addr: SocketAddr) {
        match self.nodes.iter().position(|n| n.addr == addr) {
            Some(index) => {
                let unit = self.nodes.remove(index);
                let msg = Message::UnregisterNode { addr: unit.addr };
                self.send_to_all(&msg);
                println!("Unregistered node '{}' with ip {}", unit.name, unit.addr);
            }
            None => {
                println!("Foreign/Rogue node received! {}", addr);
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
    fn ask_client_for_blender(&mut self, endpoint: Endpoint, msg: &Message) {
        // in this case, the client is asking all other client if any one have the matching blender version type.
        let _map = self
            .nodes
            .iter()
            .filter(|n| n.endpoint != endpoint)
            .map(|n| self.send_to_target(n.endpoint, &msg));
    }

    /// Generate a list of node message to send
    fn create_node_list(&self) -> Message {
        let list = self
            .nodes
            .iter()
            .map(|n| (n.addr, n.name.clone()))
            .collect();
        Message::NodeList(list)
    }

    /// send network message to specific endpoint
    fn send_to_target(&self, endpoint: Endpoint, message: &Message) {
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
    }

    /// Send message to all clients that's connected to the host.
    fn send_to_all(&self, message: &Message) {
        println!("Sending {:?} to all clients", &message);
        let data = bincode::serialize(&message).unwrap();
        for node in &self.nodes {
            self.handler.network().send(node.endpoint, &data);
        }
    }

    /// Test example: Send a example job to target node
    /// TODO: Split this function up where we'll have a method to send a new job to the server, and allowing new node to get on-going job
    // TODO: Find a way to call server.set_job to kick off the job process!
    pub fn set_job(&mut self, mut job: Job) {
        // create an example job we can use to work with this.
        if self.job.is_some() {
            panic!("Job already exists! Cannot set a new job! Need to understand how this situation can arise?");
        };

        if let Some(frame) = job.next_frame() {
            let version = job.version.clone();
            let project_file = job.project_file.clone();
            let render_queue = RenderQueue::new(frame, version, project_file, job.id);
            // If I want to get fancy and crafty - I could speed up render by splitting the frame into multiple
            // of pieces and render each quadrant per node.
            // for now let's just try this and get this working again.
            let message = Message::LoadJob(render_queue);
            self.send_to_all(&message);
        }

        self.job = Some(job);
    }
}
