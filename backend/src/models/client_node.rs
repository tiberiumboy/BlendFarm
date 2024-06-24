use crate::models::message::Message;
use anyhow::Result;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use std::collections::HashMap;
use std::net::SocketAddr;

pub struct ClientNode {
    handler: NodeHandler<()>,
    listeners: Option<NodeListener<()>>,
    name: String,
    server_endpoint: Endpoint,
    public_addr: SocketAddr,
    participants: HashMap<String, Endpoint>,
}

impl ClientNode {
    pub fn new(name: &str, port: u16) -> Result<ClientNode> {
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
            participants: HashMap::new(),
        })
    }

    pub fn run(mut self) {
        let listener = self.listeners.take().unwrap();
        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, established) => {
                if established {
                    let message = Message::RegisterNode(self.name.clone(), self.public_addr);
                    let output_data = bincode::serialize(&message).unwrap();
                    self.handler
                        .network()
                        .send(self.server_endpoint, &output_data);
                } else {
                    println!("Could not connect to the server!");
                }
            }
            NetEvent::Accepted(_, _) => unreachable!(),
            NetEvent::Message(endpoint, bytes) => self.handle_message(endpoint, bytes),
            NetEvent::Disconnected(endpoint) => {
                // TODO: What do we need to do if the node get disconnected? Should we just stop the job?
                println!("Disconnected from {}", endpoint);
            }
        })
    }

    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        let message: Message = bincode::deserialize(&bytes).unwrap();
        match message {
            Message::NodeList(nodes) => {
                println!("Node list received! ({} nodes)", nodes.len());
                for (name, addr) in nodes {
                    let text = "I see you in the participant list";
                    // self.
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
            _ => todo!("Not yet implemented!"),
        }
    }

    fn discover_nodes(&mut self, name: &str, addr: SocketAddr, text: &str) {
        let (endpoint, _) = self
            .handler
            .network()
            .connect(Transport::FramedTcp, addr)
            .unwrap();
        // self.
    }
}
