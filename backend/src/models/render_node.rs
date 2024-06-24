/*
    Developer blog:
    - Wonder if I should make this into a separate directory for network infrastructure?
*/

use crate::models::common::{ReceiverMsg, SenderMsg};
use crate::models::error::Error;
use anyhow::Result;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::NodeHandler;
use message_io::node::{self, NodeEvent, NodeListener};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::time::Duration;
use std::{net::SocketAddr, path::PathBuf, str::FromStr};

// should this be used in a separate library module?

const CHUNK_SIZE: usize = 65536;

enum Signal {
    SendChunk,
    // what else do I need to perform on network packet?
}

#[derive(Debug)]
pub struct NetworkConnection {
    handler: NodeHandler<()>,
    node_listener: Option<NodeListener<()>>,
    endpoint: Endpoint,
}

impl NetworkConnection {
    pub fn connect(host: &SocketAddr) -> Self {
        let (handler, listener) = node::split();

        let (endpoint, _) = handler
            .network()
            .connect(Transport::FrameTcp, host)
            .unwrap();

        Self {
            handler,
            endpoint,
            node_listener: Some(listener),
        }
    }

    pub fn listen(&self) {}
}

// Could I bring mutex content down here? I need to append new render node if it's discovered by the network
// Do not communicate by sharing memory
#[derive(Debug, Deserialize, Serialize)]
pub struct RenderNode {
    pub name: String,
    pub host: SocketAddr,
    // #[serde(skip)]
    // context : Box<
}

#[allow(dead_code)]
impl RenderNode {
    pub fn new(name: &str, host: SocketAddr) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            host,
        })
    }

    pub fn parse(name: &str, host: &str) -> Result<RenderNode> {
        match host.parse::<SocketAddr>() {
            Ok(host) => match Self::connect(name, host) {
                Ok(node) => Ok(node),
                Err(e) => Err(Error::PosionError(e.to_string())),
            },
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }

    fn handle_message(endpoint: Endpoint, data: &[u8]) {
        let message: ReceiverMsg = bincode::deserialize(data).unwrap();
        match message {
            ReceiverMsg::CanReceive(can) => match can {
                true => handler.signals().send(Signal::SendChunk),
                false => {
                    handler.stop();
                    println!("The receiver can not receive the file!");
                }
            },
        }
    }

    // is this something that needs ot be invoked asyncronously?
    pub fn listen(&self) -> Result<()> {
        // TODO: find out how we can establish connection here?
        let (handler, listener) = node::split();
        let (server_id, _) = handler.network().connect(Transport::FramedTcp, self.host)?;
        let mut file_bytes_sent = 0;

        listener.for_each(move |event| match event {
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(endpoint, established) => {
                    if established {
                        // Is there a way I could send out hey you establish a new rendernode connection?
                    } else {
                        println!(
                            "Can not connect to the receiver by TCP to {}",
                            endpoint.addr().ip()
                        );
                    }
                }
                // I wonder why this is unreachable?
                NetEvent::Accepted(_, _) => unreachable!(),
                NetEvent::Message(endpoint, data) => Self::handle_message(endpoint, data),
                NetEvent::Disconnected(_) => {
                    handler.stop();
                    println!("\nReceiver disconnected");
                }
            },
            NodeEvent::Signal(signal) => match signal {
                Signal::SendChunk => {
                    let mut file = File::open(file_path).unwrap();
                    let mut data = [0; CHUNK_SIZE];
                    let bytes_read = file.read(&mut data).unwrap();
                    if bytes_read > 0 {
                        let chunk = SenderMsg::Chunk(Vec::from(&data[0..bytes_read]));
                        let output_data = bincode::serialize(&chunk).unwrap();
                        handler.network().send(server_id, &output_data);
                        file_bytes_sent += bytes_read;

                        let percentage =
                            ((file_bytes_sent as f32 / file_size as f32) * 100.0) as usize;
                        println!("\rSending {:?}: {}%", file_name, percentage);

                        handler
                            .signals()
                            .send_with_timer(Signal::SendChunk, Duration::from_micros(10));
                    } else {
                        println!("\nFile sent!");
                        handler.stop();
                    }
                }
            },
        });

        Ok(())
    }

    pub fn send(self, file: &PathBuf) {
        let file_size = fs::metadata(file).unwrap().len() as usize;
        let file_name: &OsStr = file.file_name().expect("Missing file!");
        let mut file = File::open(file).unwrap();

        println!("Sender connected by TCP {}", self.endpoint.addr().ip());
        let request =
            SenderMsg::FileRequest(file_name.to_os_string().into_string().unwrap(), file_size);
        let output_data = bincode::serialize(&request).unwrap();
        self.handler.network().send(self.endpoint, &output_data);
    }

    /// Invoke the render node to start running the job
    pub fn run(self) {
        // is this where we can set the jobhandler?
        // let handler = thread::spawn(|| {});
    }

    pub fn abort(self) {}
}

impl Default for RenderNode {
    fn default() -> Self {
        // socketAddr should not crash if the host address is defined globally
        let socket = SocketAddr::from_str("127.0.0.1:15000").unwrap();
        RenderNode::new("localhost", socket).unwrap()
    }
}

impl FromStr for RenderNode {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<RenderNode>(s)
    }
}

impl PartialEq for RenderNode {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.name == other.name
    }
}

impl Drop for RenderNode {
    fn drop(&mut self) {
        self.handler.stop();
    }
}
