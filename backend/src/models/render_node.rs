use serde::{Deserialize, Serialize};
use std::net::{ SocketAddr. ToSocketAddrs};
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub id: String,
    #[serde(skip_serializing)]
    pub host: SocketAddr,
    pub name: Option<String>,
}

impl RenderNode {
    pub fn parse(name: String, host: String) -> Result<RenderNode, std::io::Error> {
        match host.to_socket_addrs() {
            Ok(host) => Ok(RenderNode {
                id: uuid::Uuid::new_v4().to_string(),
                host: host.next().unwrap(),
                name: Some(name),
            }),
            Err(e) => return Err(e),
        }
    }

    pub fn connect() -> Result<String, std::io::Error> {
        // connect to the host
        Ok("Connected".to_owned())
    }
}

impl FromStr for RenderNode {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: RenderNode = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
