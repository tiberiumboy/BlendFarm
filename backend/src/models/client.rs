/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/
use crate::models::{
    file_info::FileInfo,
    file_transfer::FileTransfer,
    message::{Message, Signal},
    node::Node,
    render_queue::RenderQueue,
    server,
};
use anyhow::Result;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::{collections::HashMap, net::SocketAddr, str::FromStr};

const CHUNK_SIZE: usize = 65536;

pub struct Client {
    handler: NodeHandler<Signal>,
    listener: Option<NodeListener<Signal>>,
    name: String,
    server_endpoint: Endpoint,
    public_addr: SocketAddr,
    // do I still need this?
    nodes: Vec<Node>,
    is_connected: bool,
    file_transfer: Option<FileTransfer>,
    // Is there a way for me to hold struct objects while performing a transfer task?
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
            file_transfer: None,
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
                            self.send_to_target(self.server_endpoint, self.register_message());
                            self.is_connected = true;
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
                        self.is_connected = false;
                        // in the case of this node disconnecting, I would like to auto renew the connection if possible.
                        // how would I go about setting this up?
                    }
                },

                // client is sending self generated signals?
                NodeEvent::Signal(signal) => match signal {
                    // Signal
                    Signal::SendChunk => {
                        let transfer = match self.file_transfer.as_mut() {
                            Some(transfer) => transfer,
                            None => return,
                        };
                        match transfer.transfer(&self.handler) {
                            Some(size) => {
                                println!("Sending {} bytes of data!", size);
                            }
                            None => {
                                println!("File transfer completed!");
                                // this means that we have completed our transfer!
                                self.file_transfer = None;
                            }
                        }
                    } //_ => todo!("Not yet implemented!"),
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
            Message::RegisterNode { name, addr } => self.add_node(&name, addr, endpoint),
            Message::UnregisterNode { addr } => self.remove_node(addr),
            Message::LoadJob(render_queue) => self.load_job(render_queue),

            // client to client
            Message::ContainBlenderResponse { have_blender } => {
                if have_blender {
                    // there should be some kind of handler to reject any other potential response
                    let msg = Message::CanReceive(true);
                    self.send_to_target(endpoint, msg);
                }
            }

            // multicast
            Message::ServerPing { port } => self.handle_server_ping(port, &endpoint),
            Message::FileRequest(file_info) => self.handle_file_request(endpoint, &file_info),
            Message::Chunk(_data) => todo!("Find a way to save data to temp?"),
            Message::CanReceive(accepted) => {
                if accepted {
                    // accept receiving chunks of data
                    // here we start sending the endpoint data?
                }
            }
            _ => println!("Unhandled client message case condition for {:?}", message),
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

    fn load_job(&mut self, render_queue: RenderQueue) {
        println!("Received a new render queue!\n{:?}", render_queue);

        // First let's check if we hvae the correct blender installation
        // then check and see if we have the files?
        if !render_queue.project_file.file_path().exists() {
            // here we will fetch the file path from the server
            // but for now let's continue.
        }

        // run the blender() - this will take some time. Could implement async/thread?
        match render_queue.run() {
            // returns frame and image path
            Ok(render_info) => {
                println!(
                    "Render completed! Sending image to server! {:?}",
                    render_info
                );

                let mut file_transfer =
                    FileTransfer::new(render_info.path.clone(), self.server_endpoint);

                file_transfer.transfer(&self.handler);
                // is there a way to convert mutable to immutable?

                self.file_transfer = Some(file_transfer);
                // wonder if there's a way to say - hey I've completed my transfer,
                // please go and look in your download folder with this exact file name,
                // then proceed to your job manager to move out to output destination.
                // first notify the server that the job is completed and prepare to receive the file
                let msg = Message::JobResult(render_info);
                self.send_to_target(self.server_endpoint, msg);

                // I need to set something to this client node? Maybe a placeholder to say "Queue to transfer"?
                self.handler.signals().send(Signal::SendChunk);

                // let msg = Message::FileRequest(info);
                // self.send_to_target(self.server_endpoint, msg);
            }
            Err(e) => println!("Fail to render on client! {:?}", e),
        }
    }

    fn handle_server_ping(&mut self, port: u16, endpoint: &Endpoint) {
        println!("Hey! Client received a multicast ping signal!");
        // Currently this is a hack and I need to find a way to get a loopback rule working.
        // TODO: find a fix for this, this is only used for testing purpose only! DO NOT SHIP!
        let server = SocketAddr::from_str(&format!("127.0.0.1:{}", port)).unwrap();
        println!("{}", endpoint);
        if !self.is_connected {
            // I am not sure why I am unable to send a register message back to the server?
            if let Ok((endpoint, _)) = self.handler.network().connect(Transport::FramedTcp, server)
            {
                self.server_endpoint = endpoint;
            }
        } else {
            println!("Sorry, we're already connected to the host!");
        }
    }

    fn handle_file_request(&mut self, endpoint: Endpoint, file_info: &FileInfo) {
        println!("name: {:?} | size: {}", file_info.path, file_info.size);
        let message = Message::CanReceive(true);
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
        // TODO: Find a way to send file from one computer to another!
    }

    fn register_message(&self) -> Message {
        Message::RegisterNode {
            name: self.name.clone(),
            addr: self.public_addr,
        }
    }

    fn send_to_target(&self, endpoint: Endpoint, message: Message) {
        println!("Sending {:?} to target [{}]", message, endpoint.addr());
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
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
