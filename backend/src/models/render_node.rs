use serde::{Deserialize, Serialize};
use std::io::Error;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub id: String,
    pub name: Option<String>,
    pub host: SocketAddr,
}

impl RenderNode {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode, Error> {
        let socket = host.parse::<SocketAddr>().unwrap();
        Ok(RenderNode {
            id: uuid::Uuid::new_v4().to_string(),
            name: Some(name.to_owned()),
            host: socket,
        })
    }

    pub fn connect(&self) -> Result<String, Error> {
        // connect to the host
        Ok("Connected".to_owned())
    }
}

impl FromStr for RenderNode {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Error> {
        let obj: RenderNode = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
