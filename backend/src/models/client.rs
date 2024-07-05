/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/

use crate::models::{message::Message, node::Node, server};
use anyhow::{Error, Result};
// use gethostname::gethostname;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

use super::job::Job;

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
                    NetEvent::Message(endpoint, bytes) => {
                        if let Err(err) = self.handle_message(endpoint, bytes) {
                            println!("{}", err);
                        }
                    }
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

    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) -> Result<()> {
        // why did this part failed?
        let message: Message = match bincode::deserialize(bytes) {
            Ok(data) => data,
            Err(e) => return Err(Error::new(e)),
        };

        match message {
            // Client receives this message from the server
            Message::NodeList(nodes) => self.handle_node_list(nodes, endpoint),
            // how did I do this part again? Let's review over to message io
            Message::FileRequest(name, size) => self.handle_file_request(endpoint, &name, size),
            Message::Chunk(_data) => todo!("Find a way to save data to temp?"),
            Message::ServerPing => {
                println!("Hey! Client received a multicast ping signal!");
                if !self.is_connected {
                    self.send_to_target(endpoint, &self.register_message());
                } else {
                    println!("Sorry, we're already connected to the host!");
                }
            }

            Message::RegisterNode { name, addr } => self.add_node(&name, addr, endpoint),
            Message::UnregisterNode { addr } => self.remove_node(addr),
            Message::LoadJob(job) => self.load_job(job),
            _ => todo!("Not yet implemented!"),
        };
        Ok(())
    }

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

    fn send_to_target(&self, endpoint: Endpoint, message: &Message) {
        println!("Sending {:?} to target [{}]", message, endpoint.addr());
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
    }

    fn send_to_server(&self, message: &Message) {
        println!("Sending {:?} to server", message);
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
