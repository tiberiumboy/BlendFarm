/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/

use crate::models::{message::Message, node::Node};
use anyhow::Result;
use gethostname::gethostname;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::collections::HashMap;
use std::net::SocketAddr;

pub struct Client {
    handler: NodeHandler<()>,
    listener: Option<NodeListener<()>>,
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
            listener: Some(listener),
            name: name.to_string(),
            server_endpoint: endpoint,
            public_addr: listen_addr,
            nodes: Vec::new(),
        })
    }

    // Client begin listening for server
    pub fn run(mut self) {
        // This doesn't seem like it will repeat?
        // TODO: How do I make it broadcast so it'll repeat?

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
                        self.listen();
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

    // wait to receive a multi-cast response.
    // this will send register node notification to the signal.
    pub fn listen(&mut self) {
        let multicast_addr = "239.255.0.1:3010";
        println!("Begin listening broadcast signals on [{multicast_addr}]");
        // let (endpoint, _) = self
        //     .handler
        //     .network()
        //     .connect(Transport::Udp, multicast_addr)
        //     .unwrap();

        // let listener = self.listeners.take().unwrap();

        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _always_true_for_udp) => {}
            _ => {
                println!("Found something from listener?")
            }
        })
    }

    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        let message: Message = bincode::deserialize(bytes).unwrap();
        match message {
            // Client receives this message from the server
            Message::NodeList(nodes) => self.handle_node_list(nodes, endpoint),
            // how did I do this part again? Let's review over to message io
            Message::FileRequest(name, size) => {
                let message = Message::CanReceive(true);
                let data = bincode::serialize(&message).unwrap();
                self.handler.network().send(endpoint, &data);
            }
            Message::Chunk(data) => {}

            Message::RegisterNode { name, addr } => self.add_node(&name, addr, endpoint),
            Message::UnregisterNode { addr } => self.remove_node(addr),
            _ => todo!("Not yet implemented!"),
        }
    }

    fn handle_node_list(&mut self, nodes: HashMap<SocketAddr, String>, endpoint: Endpoint) {
        // how come I don't receive event for this one?
        println!("Node list received! ({} nodes)", nodes.len());
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

    fn register_message(&self) -> Message {
        println!("Creating register message.");
        Message::RegisterNode {
            name: self.name.clone(),
            addr: self.public_addr,
        }
    }

    #[allow(dead_code)]
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
