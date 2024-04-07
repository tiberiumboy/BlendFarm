use crate::models::error::Error;
use crate::services::sender;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Idle,
    Running(u8),
    Error(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub id: String,
    pub status: NodeStatus,
    pub name: Option<String>,
    pub host: SocketAddr,
}

#[allow(dead_code)]
impl RenderNode {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode, Error> {
        match host.parse::<SocketAddr>() {
            Ok(socket) => Ok(RenderNode {
                id: uuid::Uuid::new_v4().to_string(),
                name: Some(name.to_owned()),
                status: NodeStatus::Idle,
                host: socket,
            }),
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }

    pub fn connect(&self) -> Result<String, Error> {
        // connect to the host
        Ok("Connected".to_owned())
    }

    #[allow(dead_code)]
    pub fn send(&self, file: &PathBuf) {
        sender::send(file, self);
    }
}

impl FromStr for RenderNode {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<RenderNode>(s)
    }
}
