/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/
use crate::models::{
    file_info::FileInfo,
    file_transfer::FileTransfer,
    message::{Message, Signal},
    render_queue::RenderQueue,
    server,
};
use anyhow::Result;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeEvent, NodeHandler, NodeListener};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use super::server::MULTICAST_ADDR;

// const CHUNK_SIZE: usize = 65536;

pub struct Client {
    handler: NodeHandler<Signal>,
    listener: Option<NodeListener<Signal>>,
    name: String,
    server_endpoint: Option<Endpoint>,
    public_addr: SocketAddr,
    file_transfer: Option<FileTransfer>,
    // Is there a way for me to hold struct objects while performing a transfer task?
}

impl Client {
    pub fn new(name: &str) -> Result<Client> {
        let (handler, listener) = node::split();

        // this would error if I am not connected to the internet. I need this to be able to run despite not connected to anything ( run as local host!)
        let ip = local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let public_addr = SocketAddr::new(ip, 0);

        // this will handle the multicast address channel
        handler
            .network()
            .listen(Transport::Udp, server::MULTICAST_ADDR)?;

        // connect to multicast address
        if let Ok((endpoint, _)) = handler.network().connect(Transport::Udp, MULTICAST_ADDR) {
            // send client ping to multicast address
            let name = name.to_owned();
            let msg = Message::ClientPing { name };
            let data = bincode::serialize(&msg).unwrap();
            handler.network().send(endpoint, &data);
        }

        // setup listener for client
        // I only got socket address from this?
        let listen_addr = match handler.network().listen(Transport::FramedTcp, public_addr) {
            Ok(conn) => conn.1, // don't care about Resource Id (0)
            Err(e) => {
                println!("Error listening to address [{}]! {:?}", public_addr, e);
                return Err(anyhow::anyhow!("Error listening to port! {:?}", e));
            }
        };

        Ok(Self {
            handler,
            listener: Some(listener),
            name: name.to_string(),
            server_endpoint: None,
            public_addr: listen_addr,
            file_transfer: None,
        })
    }

    // Client begin listening for server
    pub fn run(mut self) {
        let listener = self.listener.take().unwrap();
        listener.for_each(move |event| {
            match event {
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Connected(endpoint, established) => {
                        self.handle_connected(endpoint, established)
                    }
                    NetEvent::Accepted(endpoint, _) => self.handle_accepted(endpoint),
                    NetEvent::Message(endpoint, bytes) => self.handle_message(endpoint, bytes),
                    NetEvent::Disconnected(endpoint) => self.handle_disconnected(endpoint),
                },

                // client is sending self generated signals?
                NodeEvent::Signal(signal) => match signal {
                    // Signal
                    Signal::SendChunk => self.handle_sending_chunk(),
                    //_ => todo!("Not yet implemented!"),
                },
            }
        })
    }

    // maybe we don't need this at all?
    fn handle_connected(&mut self, endpoint: Endpoint, established: bool) {
        if established {
            println!("Node connected! [{}]", endpoint.addr());
            self.server_endpoint = Some(endpoint);
        } else {
            println!("Could not connect to the server!?? {}", endpoint);
            // is there any way I could just begin the listen process here?
        }
    }

    fn handle_accepted(&mut self, endpoint: Endpoint) {
        // an tcp connection accepts the connection!
        println!("Client was accepted to server! [{}]", endpoint.addr());
        self.server_endpoint = Some(endpoint);
    }

    fn handle_message(&mut self, endpoint: Endpoint, bytes: &[u8]) {
        // why did this part failed?
        let message: Message = match bincode::deserialize(bytes) {
            Ok(data) => data,
            // just for now we'll just panic. making the assumption that both side should have identical data type matches, it should be fine.
            Err(e) => panic!("Error deserializing message input: \n{:?}", e),
        };

        match message {
            // server to client
            Message::ServerPing => self.server_ping(endpoint),
            Message::LoadJob(render_queue) => self.load_job(render_queue),
            Message::CancelJob => self.cancel_job(),

            // client to client
            Message::ClientPing { name: _ } => self.client_ping(),
            Message::ContainBlenderResponse { have_blender } => {
                self.contain_blender_response(endpoint, have_blender)
            }

            // multicast
            Message::FileRequest(file_info) => self.file_request(endpoint, &file_info),
            Message::Chunk(_data) => todo!("Find a way to save data to temp?"),
            Message::CanReceive(accepted) => self.can_receive(accepted),
            _ => println!("Unhandled client message case condition for {:?}", message),
        };
    }

    fn handle_disconnected(&mut self, endpoint: Endpoint) {
        // TODO: How can we initialize another listening job? We definitely don't want the user to go through the trouble of figuring out which machine has stopped.
        // Disconnected was call when server was shut down
        println!("Lost connection to host! [{}]", endpoint.addr());
        self.server_endpoint = None;
        // in the case of this node disconnecting, I would like to auto renew the connection if possible.
    }

    fn handle_sending_chunk(&mut self) {
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
                // assuming that we have connection to the server? otherwise rendering job should abort immediately.
                let endpoint = self.server_endpoint.unwrap();

                println!(
                    "Render completed! Sending image to server! {:?}",
                    render_info
                );

                let mut file_transfer = FileTransfer::new(render_info.path.clone(), endpoint);

                file_transfer.transfer(&self.handler);
                // is there a way to convert mutable to immutable?

                self.file_transfer = Some(file_transfer);
                // wonder if there's a way to say - hey I've completed my transfer,
                // please go and look in your download folder with this exact file name,
                // then proceed to your job manager to move out to output destination.
                // first notify the server that the job is completed and prepare to receive the file
                let msg = Message::JobResult(render_info);
                self.send_to_target(endpoint, msg);

                // I need to set something to this client node? Maybe a placeholder to say "Queue to transfer"?
                self.handler.signals().send(Signal::SendChunk);

                // let msg = Message::FileRequest(info);
                // self.send_to_target(self.server_endpoint, msg);
            }
            Err(e) => println!("Fail to render on client! {:?}", e),
        }
    }

    fn cancel_job(&self) {
        println!("Cancel active job!");
    }

    fn contain_blender_response(&self, endpoint: Endpoint, have_blender: bool) {
        if !have_blender {
            println!(
                "Client [{}] does not have the right version of blender installed",
                endpoint.addr()
            );
            return;
        }

        self.send_to_target(endpoint, Message::CanReceive(true));
    }

    /// Handle server multi-cast ping signal
    fn server_ping(&mut self, endpoint: Endpoint) {
        println!(
            "Hey! Client received a multicast ping signal! {}",
            &endpoint.addr()
        );

        if self.server_endpoint.is_some() {
            println!("Sorry, we're already connected to the host!");
            return;
        }

        self.send_to_target(endpoint.clone(), self.register_message());
        self.server_endpoint = Some(endpoint);
    }

    fn client_ping(&self) {
        println!("Received client ping! Ignoring!");
    }

    fn file_request(&mut self, endpoint: Endpoint, file_info: &FileInfo) {
        println!("name: {:?} | size: {}", file_info.path, file_info.size);
        let message = Message::CanReceive(true);
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
        // TODO: Find a way to send file from one computer to another!
    }

    fn can_receive(&self, accepted: bool) {
        if !accepted {
            println!("Client cannot accept file transfer!");
            return;
        };

        self.handler.signals().send(Signal::SendChunk);
    }

    fn register_message(&self) -> Message {
        Message::RegisterNode {
            name: self.name.clone(),
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
        if let Some(endpoint) = self.server_endpoint {
            let message = Message::UnregisterNode {
                addr: self.public_addr,
            };
            println!("Sending unregisternode packet to host before stopping!");
            self.send_to_target(endpoint, message);
        }

        println!("Stopping connection!");
        self.handler.stop();
    }
}
