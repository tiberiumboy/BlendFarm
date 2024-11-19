/*
the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.

*/
// use libp2p::connection_limits::Behaviour;
use libp2p::{futures::StreamExt, ping::Behaviour};
use libp2p::multiaddr::Protocol;
use libp2p::swarm::SwarmEvent;
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, RwLock}};
use thiserror::Error;
use libp2p::{noise, ping, tcp, yamux, Multiaddr, Swarm, SwarmBuilder};

// Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
// pub const MULTICAST_ADDR: &str = "239.255.0.1:3010";
pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
pub const MULTICAST_PORT: u16 = 3010;
pub const MULTICAST_SOCK: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_ADDR), MULTICAST_PORT);
pub const CHUNK_SIZE: usize = 65536;

pub type Port = u16;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Not Connected")]
    NotConnected,
    #[error("Invalid connection")]
    Invalid,
    #[error("Bad Input")]
    BadInput,
}

#[derive(Serialize, Deserialize)]
pub enum UdpMessage {
    Ping {
        host: bool,
        addr: SocketAddr,
        name: String,
    },
}

impl UdpMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(&data)
    }
}

pub struct NetworkService {
    // this way we can determine if we're active or not.
    is_host: bool,
    swarm: Arc<RwLock<Option<Swarm<Behaviour>>>>,
}

impl NetworkService {
    pub fn new(is_host: bool) -> Self {
        Self {
            is_host,
            swarm: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect(&mut self, port: Port) -> Result<(), Box<dyn std::error::Error>> {
        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(libp2p::tcp::Config::default(),
             libp2p::tls::Config::new, 
             yamux::Config::default)?
             .with_behaviour(|_| ping::Behaviour::default())?
             .build();
        let mut addr: Multiaddr ="/ip4/0.0.0.0/".parse()?;
        addr.push(Protocol::Tcp(port));
        swarm.listen_on(addr)?;
        self.swarm = Arc::new(RwLock::new(Some(swarm)));
        
        // hmm This could be a problem?
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
                SwarmEvent::Behaviour(event) => println!("{event:?}"),
                _ => {}
            }
        }
        
        // Ok(())
    }
}
