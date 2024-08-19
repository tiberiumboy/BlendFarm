/*
    Developer blog:
    - Wonder if I should make this into a separate directory for network infrastructure?
*/
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, str::FromStr};

#[derive(Debug, Deserialize, Serialize)]
pub struct RenderNode {
    pub name: String,
    pub host: SocketAddr,
}

// this code may be dead?
impl RenderNode {
    pub fn new(name: &str, host: SocketAddr) -> Self {
        Self {
            name: name.to_string(),
            host,
        }
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

// impl Drop for RenderNode {
//     fn drop(&mut self) {
//         self.handler.stop();
//     }
// }
