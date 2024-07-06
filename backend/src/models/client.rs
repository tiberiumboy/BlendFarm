/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/

use crate::models::server_setting::ServerSetting;
use crate::models::{job::Job, message::Message, node::Node, server};
use anyhow::{Error, Result};
use blender::{args::Args, blender::Blender};
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

pub struct Client {
    handler: NodeHandler<()>,
    listener: Option<NodeListener<()>>,
    name: String,
    server_endpoint: Endpoint,
    public_addr: SocketAddr,
    nodes: Vec<Node>,
    is_connected: bool,
}

impl Client {
    pub fn new(name: &str, port: u16) -> Result<Client> {
        let (handler, listener) = node::split();

        let listen_addr = "127.0.0.1:0";
        let (_, listen_addr) = handler
            .network()
            .listen(Transport::FramedTcp, listen_addr)?;

        let discovery_addr = format!("127.0.0.1:{}", port);
        let (endpoint, _) = handler
            .network()
            .connect(Transport::FramedTcp, discovery_addr)?;

        // this will handle the multicast address channel
        handler
            .network()
            .listen(Transport::Udp, server::MULTICAST_ADDR)?;

        Ok(Self {
            handler,
            listener: Some(listener),
            name: name.to_string(),
            server_endpoint: endpoint,
            public_addr: listen_addr,
            nodes: Vec::new(),
            is_connected: false,
        })
    }

    // Client begin listening for server
    pub fn run(mut self) {
        let listener = self.listener.take().unwrap();
        listener.for_each(move |event| {
            match event {
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Connected(_, established) => {
                        if established {
                            println!("Node connected! Sending register node!");
                            self.send_to_server(&self.register_message());
                        } else {
                            println!("Could not connect to the server!");
                            // is there any way I could just begin the listen process here?
                        }
                    }
                    NetEvent::Accepted(_, _) => unreachable!(),
                    NetEvent::Message(endpoint, bytes) => self.handle_message(endpoint, bytes),
                    NetEvent::Disconnected(endpoint) => {
                        // TODO: How can we initialize another listening job? We definitely don't want the user to go through the trouble of figuring out which machine has stopped.
                        // Disconnected was call when server was shut down
                        println!("Lost connection to host! [{}]", endpoint.addr());
                        // in the case of this node disconnecting, I would like to auto renew the connection if possible.
                        // how would I go about setting this up?
                    }
                },

                // client is sending self generated signals?
                NodeEvent::Signal(signal) => match signal {
                    // Signal
                    _ => todo!("Not yet implemented!"),
                },
            }
        })
    }

    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        // why did this part failed?
        let message: Message = match bincode::deserialize(bytes) {
            Ok(data) => data,
            // just for now we'll just panic. making the assumption that both side should have identical data type matches, it should be fine.
            Err(e) => panic!("Error deserializing message input: \n{:?}", e),
        };

        match message {
            // Client receives this message from the server
            Message::NodeList(nodes) => self.handle_node_list(nodes, endpoint),
            // how did I do this part again? Let's review over to message io
            Message::FileRequest(name, size) => self.handle_file_request(endpoint, &name, size),
            Message::Chunk(_data) => todo!("Find a way to save data to temp?"),
            Message::ServerPing { port } => {
                println!("Hey! Client received a multicast ping signal!");
                let server = SocketAddr::from_str(&format!("127.0.0.1:{}", port)).unwrap();
                // let mut server = endpoint.addr().clone();
                // server.set_port(port);
                if !self.is_connected {
                    // I am not sure why I am unable to send a register message back to the server?
                    let (endpoint, _) = self
                        .handler
                        .network()
                        .connect(Transport::FramedTcp, server)
                        .unwrap(); // should in theory be able to connect back to the server?
                    println!(
                        "{:?} | {:?}",
                        &self.server_endpoint.addr(),
                        &endpoint.addr()
                    );
                    self.server_endpoint = endpoint;
                } else {
                    println!("Sorry, we're already connected to the host!");
                }
            }

            Message::RegisterNode { name, addr } => self.add_node(&name, addr, endpoint),
            Message::UnregisterNode { addr } => self.remove_node(addr),
            Message::LoadJob(job) => self.load_job(job),
            _ => todo!("Not yet implemented!"),
        };
    }

    // this function takes two parts. it handles the incoming list, and change the struct field is_connected to true.
    // we're making a fine assumption that once we received the node back from the server, it means that the server have acknoledge the response.
    // TODO: See if this is a bad programming practice? Should I keep single responsibility implementation?
    fn handle_node_list(&mut self, nodes: HashMap<SocketAddr, String>, endpoint: Endpoint) {
        // how come I don't receive event for this one?
        println!("Node list received! ({} nodes)", nodes.len());
        self.is_connected = true;
        for (addr, name) in nodes {
            let node = Node::new(&name, addr, endpoint);
            self.nodes.push(node);
            println!("{} [{}] is online", name, addr);
        }
    }

    fn add_node(&mut self, name: &str, addr: SocketAddr, endpoint: Endpoint) {
        let node = Node::new(name, addr, endpoint);
        self.nodes.push(node);
        println!("{} [{}] connected to the server", name, addr);
    }

    fn remove_node(&mut self, addr: SocketAddr) {
        if let Some(index) = self.nodes.iter().position(|n| n.addr == addr) {
            let element = self.nodes.remove(index);
            println!("{} [{}] Disconnected", element.name, element.addr);
        }
    }

    fn load_job(&mut self, job: Job) {
        println!("Received a new job!\n{:?}", job);

        // First let's check if we hvae the correct blender installation
        // then check and see if we have the files?
        if !job.project_file.file_path().exists() {
            // here we will fetch the file path from the server
            // but for now let's continue.
        }

        let mut config = ServerSetting::load();
        let blender = config.get_blender(job.version);
        let args = Args::new(job.project_file.file_path(), job.output, job.mode);
        match blender.render(&args) {
            Ok(str) => println!("{}", str),
            Err(e) => println!("{}", e),
        }
    }

    fn handle_file_request(&mut self, endpoint: Endpoint, name: impl AsRef<Path>, size: usize) {
        println!("name: {:?} | size: {}", name.as_ref().as_os_str(), size);
        let message = Message::CanReceive(true);
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
    }

    fn register_message(&self) -> Message {
        Message::RegisterNode {
            name: self.name.clone(),
            addr: self.public_addr,
        }
    }

    // TODO: We will use this for blender installation transfer, but for now this isn't important for our objective goal
    #[allow(dead_code)]
    fn send_to_target(&self, endpoint: Endpoint, message: &Message) {
        println!("Sending {:?} to target [{}]", message, endpoint.addr());
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
    }

    fn send_to_server(&self, message: &Message) {
        println!(
            "Sending {:?} to server [{}]",
            message,
            self.server_endpoint.addr()
        );
        let data = bincode::serialize(message).unwrap();
        self.handler.network().send(self.server_endpoint, &data);
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        println!("About to drop!");
        let message = Message::UnregisterNode {
            addr: self.public_addr,
        };
        let output_data = bincode::serialize(&message).unwrap();

        println!("Sending unregisternode packet to host before stopping!");
        self.handler
            .network()
            .send(self.server_endpoint, &output_data);

        println!("Stopping connection!");
        self.handler.stop();
    }
}
