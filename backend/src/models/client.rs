use crate::models::{message::Message, node::Node};
use anyhow::Result;
use gethostname::gethostname;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::collections::HashMap;
use std::net::SocketAddr;

pub struct Client {
    handler: NodeHandler<()>,
    listeners: Option<NodeListener<()>>,
    name: String,
    server_endpoint: Endpoint,
    public_addr: SocketAddr,
    nodes: Vec<Node>,
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

        Ok(Self {
            handler,
            listeners: Some(listener),
            name: name.to_string(),
            server_endpoint: endpoint,
            public_addr: listen_addr,
            nodes: Vec::new(),
        })
    }

    // Client begin listening for server
    pub fn run(mut self) {
        let listener = self.listeners.take().unwrap();
        listener.for_each(move |event| {
            match event {
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Connected(_, established) => {
                        if established {
                            println!("Node connected! Sending register node!");
                            self.send_to_server(&self.register_message());
                        } else {
                            println!("Could not connect to the server!");
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
        let message: Message = bincode::deserialize(&bytes).unwrap();
        match message {
            // Client receives this message from the server
            Message::NodeList(nodes) => {
                // how come I don't receive event for this one?
                println!("Node list received! ({} nodes)", nodes.len());
                for (addr, name) in nodes {
                    let node = Node::new(&name, addr, endpoint);
                    self.nodes.push(node);
                    println!("{} [{}] is online", name, addr);
                }
            }
            Message::FileRequest(name, size) => {
                let message = Message::CanReceive(true);
                let output_data = bincode::serialize(&message).unwrap();
                // how would I go about sending files?
                // if let Some(listener) = self.listeners {
                //     self.handler
                //         .network()
                //         .send(listener.get(&name).unwrap(), &output_data);
                // }
            }
            Message::Chunk(data) => {}
            Message::RegisterNode { name, addr } => self.add_node(&name, addr, endpoint),
            Message::UnregisterNode { addr } => self.remove_node(addr),
            _ => todo!("Not yet implemented!"),
        }
    }

    // fn discover_nodes(&mut self, name: &str, addr: SocketAddr, text: &str) {
    //     let (endpoint, _) = self
    //         .handler
    //         .network()
    //         .connect(Transport::FramedTcp, addr)
    //         .unwrap();
    //     // self.
    // }

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

    fn register_message(&self) -> Message {
        println!("Creating register message.");
        Message::RegisterNode {
            name: self.name.clone(),
            addr: self.public_addr,
        }
    }

    fn send_to_server(&self, message: &Message) {
        println!("Sending {:?} to server", message);
        let data = bincode::serialize(message).unwrap();
        self.handler.network().send(self.server_endpoint, &data);
    }
}

impl Default for Client {
    fn default() -> Self {
        let hostname = gethostname().into_string().unwrap();
        Self::new(&hostname, 15000).unwrap()
    }
}

// impl Drop for Node {
//     fn drop(&mut self) {
//         println!("About to drop!");
//         let message = Message::UnregisterNode {
//             addr: self.public_addr,
//         };
//         let output_data = bincode::serialize(&message).unwrap();

//         println!("Sending unregisternode packet to host before stopping!");
//         self.handler
//             .network()
//             .send(self.server_endpoint, &output_data);

//         println!("Stopping connection!");
//         self.handler.stop();
//     }
// }
