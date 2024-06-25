use crate::models::message::Message;
use anyhow::Result;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::net::SocketAddr;

pub struct Server {
    handler: NodeHandler<()>,
    listeners: Option<NodeListener<()>>,
    nodes: Vec<Unit>,
}

// TODO: I'm worry about name ambiguous here
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Unit {
    name: String,
    addr: SocketAddr,
    endpoint: Endpoint,
}

impl Unit {
    fn new(name: &str, addr: SocketAddr, endpoint: Endpoint) -> Self {
        Self {
            name: name.to_string(),
            addr,
            endpoint,
        }
    }
}

impl Server {
    pub fn new(port: u16) -> Result<Server> {
        let (handler, listeners) = node::split();
        let listen_addr = format!("127.0.0.1:{}", port);
        handler
            .network()
            .listen(Transport::FramedTcp, listen_addr)?;

        Ok(Self {
            handler,
            listeners: Some(listeners),
            nodes: Vec::new(),
        })
    }

    pub fn run(mut self) {
        let listener = self.listeners.take().unwrap();
        listener.for_each(move |event| match event {
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(_, _) => unreachable!(),
                NetEvent::Accepted(_, _) => (),
                NetEvent::Message(endpoint, bytes) => self.handle_message(endpoint, bytes),
                NetEvent::Disconnected(endpoint) => self.unregister(&endpoint.addr()),
            },
            NodeEvent::Signal(_signal) //=> match signal {
                // Signal::SendChunk => self.send_chunk(),
                => println!("Signal received, but not implemented!"),
            //},
        });
    }

    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        let msg: Message = bincode::deserialize(bytes).unwrap();

        match msg {
            Message::RegisterNode { name, addr } => {
                self.send_list(&endpoint);
                self.register(&name, addr, endpoint)
            }
            Message::UnregisterNode { addr } => self.unregister(&addr),
            Message::LoadJob() => {
                // this will begin the job,
                // first I need to fetch the file from the server
                //
            }
            _ => todo!("Not yet implemented!"),
        }
    }

    fn send_list(&self, endpoint: &Endpoint) {
        let list = self
            .nodes
            .iter()
            .map(|node| (node.name.clone(), node.addr))
            .collect();
        let message = Message::NodeList(list);
        let output_data = bincode::serialize(&message).unwrap();
        self.handler.network().send(*endpoint, &output_data);
    }

    fn register(&mut self, name: &str, addr: SocketAddr, endpoint: Endpoint) {
        let node = Unit::new(name, addr, endpoint);
        let message = Message::RegisterNode {
            name: name.to_string(),
            addr,
        };
        let output_data = bincode::serialize(&message).unwrap();

        for node in &mut self.nodes {
            self.handler.network().send(node.endpoint, &output_data);
        }

        self.nodes.push(node);
        println!("Registering new node '{}' with ip {}", name, addr);
    }

    fn unregister(&mut self, addr: &SocketAddr) {
        println!("{}", addr);
        match self.nodes.iter().position(|x| x.addr == *addr) {
            Some(position) => {
                let node = self.nodes.remove(position);
                println!("Unregistering node '{}'", node.name);
                let message = Message::UnregisterNode { addr: node.addr };
                let output_data = bincode::serialize(&message).unwrap();
                for node in &mut self.nodes {
                    self.handler.network().send(node.endpoint, &output_data);
                }
                println!("Unregistered node '{}' with ip {}", node.name, node.addr);
            }
            None => println!("Node not found!"),
        };
    }
}
