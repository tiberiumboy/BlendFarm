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

const CHUNK_SIZE: usize = 65536;

// Not sure if I need the state?
#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
#[derive(Debug, Serialize, Deserialize)]
pub struct Running;
#[derive(Debug)]
pub struct Inactive;

enum Signal {
    SendChunk,
    // what else do I need to perform on network packet?
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub name: String,
    pub host: SocketAddr,
    handler: NodeHandler<()>,
    endpoint: Endpoint,
    node_listener: Option<NodeListener<()>>,
}

#[allow(dead_code)]
impl RenderNode {
    pub fn new(name: &str, host: SocketAddr) -> Self {
        let (handler, node_listener) = node::split();

        let listen_addr = "127.0.0.1:0";
        let (_, listen_addr) = handler.network().connect(Transport::FramedTcp, 
        Self {
            name: name.to_string(),
            host,
            handler: None,
        }
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

    pub fn host() -> Result<RenderNode> {
        let (handler, listener) = node::split();
    }

    pub fn listen(&self) -> Result<()> {
        // TODO: find out how we can establish connection here?
        let (handler, listener) = node::split();
        let (server_id, _) = handler.network().connect(Transport::FramedTcp, self.host)?;

        listener.for_each(move |event| match event {
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(endpoint, established) => {
                    if established {
                    } else {
                        println!(
                            "Can not connect to the receiver by TCP to {}",
                            endpoint.addr().ip()
                        );
                    }
                }
                NetEvent::Accepted(_, _) => unreachable!(),
                NetEvent::Message(_, input_data) => {
                    let message: ReceiverMsg = bincode::deserialize(input_data).unwrap();
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
                // NetEvent::PauseJob(_) => {}
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
    }

    pub fn send(file_path: &PathBuf, target: &RenderNode) {
        let file_size = fs::metadata(file_path).unwrap().len() as usize;
        let mut file = File::open(file_path).unwrap();
        let file_name: &OsStr = file_path.file_name().expect("Missing file!");

        let mut file_bytes_sent = 0;
    }

    #[allow(dead_code)]
    pub fn disconnected(self) {}

    #[allow(dead_code)]
    pub fn send(self, file: &PathBuf) {
        println!("Sender connected by TCP {}", endpoint.addr().ip());
        let request =
            SenderMsg::FileRequest(file_name.to_os_string().into_string().unwrap(), file_size);
        let output_data = bincode::serialize(&request).unwrap();
        handler.network().send(endpoint, &output_data);
    }

    /// Invoke the render node to start running the job
    pub fn run(self) {
        // is this where we can set the jobhandler?
        // let handler = thread::spawn(|| {});
    }

    pub fn abort(self) {}
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
