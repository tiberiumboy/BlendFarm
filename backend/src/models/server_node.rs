use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};

pub struct ServerNode {
    handler: NodeHandler<()>,
    listeners: Option<NodeListener<()>>,
    nodes: HashMap<String, Node>,
}

struct Node {
    addr: SocketAddr,
    endpoint: Endpoint,
}

impl ServerNode {
    pub fn new(port: u16) -> Result<ServerNode> {
        let (handler, listener) = node::split();
        let listen_addr = !format("127.0.0.1:{}", port);
        handler
            .network()
            .listen(Transport::FramedTcp, listen_addr)?;

        Ok(Self {
            handler,
            listeners,
            nodes: HashMap::new(),
        })
    }

    pub fn run(mut self) {
        let listener = self.listener.take().unwrap();
        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => unreachable!(),
            NetEvent::Accepted(_, _) => (),
            NetEvent::Message(endpoint, data) => {
                let msg: Message = bincode::deserialize(&bytes).unwrap();
                match msg {
                    Message::RegisterNode(name, addr) => {
                        self.register(name, addr, endpoint);
                    },
                    Message::UnregisterNode(name) => {
                        self.nodes.remove(&name);
                    },

                    Message::LoadJob() => {
                        // this will begin the job,
                        // first I need to fetch the file from the server
                        //
                    },
                    Message::
                }
            }
            NetEvent::Disconnected(endpoint) => {
                // I mean is this necessary? What if we just wanted to update status? Keep historical information?
                // TODO: keep archive of last known successful node connection instead of deleting history.
                self.nodes.retain(|_, node| node.endpoint != endpoint);
            }
        })
    }

    fn register(&mut self, name: &str, addr: SocketAddr, endpoint: Endpoint) {
        if !self.nodes.contains_key(name) {
            let list = self
                .nodes
                .iter()
                .map(|(name, info)| (name.clone(), info.addr))
                .collect();

            let message = Message::ModeList(list);
            let output_data = bincode::serialize(&message).unwrap();
            for node in &mut self.nodes {
                self.handler.network().send(node.1.endpoint, &output_data);
            }
            self.handler.network().send(endpoint, &output_data);

            self.nodes.insert(name.to_string(), endpoint);
        }
    }

    fn unregister(&mut self, name: &str) {
        if let Some(info) = self.nodes.remove(name) {
            let message = Message::UnregisterNode(name.to_string());
            let output_data = bincode::serialize(&message).unwrap();
            for node in &mut self.nodes {
                self.handler.network().send(node.1.endpoint, &output_data);
            }
            println!("Removed node '{}' with ip {}", name, info.addr);
        } else {
            println!(
                "Cannot unregister an non-existent node with name '{}'",
                name
            );
        }
    }
}
