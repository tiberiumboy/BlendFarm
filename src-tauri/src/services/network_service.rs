/*
the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.

*/
use libp2p::swarm::dummy;
use libp2p::Multiaddr;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use thiserror::Error;

// Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
// pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
// pub const MULTICAST_PORT: u16 = 3010;
// pub const MULTICAST_SOCK: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_ADDR), MULTICAST_PORT);

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

#[derive(Default)]
pub struct BlendFarmBehaviour {}

// impl NetworkBehaviour for BlendFarmBehaviour {
//     type ConnectionHandler;
//     type ToSwarm;

//     fn handle_established_inbound_connection(
//         &mut self,
//         _connection_id: ConnectionId,
//         peer: PeerId,
//         local_addr: &Multiaddr,
//         remote_addr: &Multiaddr,
//     ) -> Result<THandler<Self>, ConnectionDenied> {
//         todo!()
//     }

//     fn handle_established_outbound_connection(
//         &mut self,
//         _connection_id: ConnectionId,
//         peer: PeerId,
//         addr: &Multiaddr,
//         role_override: Endpoint,
//     ) -> Result<THandler<Self>, ConnectionDenied> {
//         todo!()
//     }

//     fn on_swarm_event(&mut self, event: FromSwarm) {
//         todo!()
//     }

//     fn on_connection_handler_event(
//         &mut self,
//         _peer_id: PeerId,
//         _connection_id: ConnectionId,
//         _event: THandlerOutEvent<Self>,
//     ) {
//         todo!()
//     }

//     fn poll(
//         &mut self,
//         cx: &mut Context<'_>,
//     ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
//         todo!()
//     }
// }

// this will help launch libp2p network. Should use QUIC whenever possible!
pub struct NetworkService {}

impl NetworkService {
    pub async fn new(timeout: u64) -> Result<(), Box<dyn std::error::Error>> {
        println!("new has been called");

        let tcp_config = libp2p::tcp::Config::default();
        let udp: Multiaddr = "/ip4/0.0.0.0/tcp/15000".parse()?;
        let duration = Duration::from_secs(timeout);

        println!("Building a swarm");
        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp_config,
                libp2p::tls::Config::new,
                libp2p::yamux::Config::default,
            )?
            // .with_behaviour(|_| BlendFarmBehaviour::default())?
            .with_behaviour(|_| dummy::Behaviour)?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
            .build();

        println!("Listening on {udp:?}");

        swarm.listen_on(udp.clone())?;
        // What's the difference here?
        // swarm.dial(udp)?;
        println!("About to dial as random!");
        swarm.dial(PeerId::random())?; // I wonder what this one will do? :think:?

        // Ok(swarm)
        Ok(())
    }
}
