use message_io::network::Endpoint;
use std::net::SocketAddr;

// TODO: I'm worry about name ambiguous here
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct Node {
    pub name: String,
    pub addr: SocketAddr,
    pub endpoint: Endpoint,
}

impl Node {
    pub fn new(name: &str, addr: SocketAddr, endpoint: Endpoint) -> Self {
        Self {
            name: name.to_string(),
            addr,
            endpoint,
        }
    }
}