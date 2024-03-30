use crate::models::error::Error;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;

use super::project_file::ProjectFile;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub id: String,
    pub name: Option<String>,
    pub host: SocketAddr,
}

impl RenderNode {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode, Error> {
        match host.parse::<SocketAddr>() {
            Ok(socket) => Ok(RenderNode {
                id: uuid::Uuid::new_v4().to_string(),
                name: Some(name.to_owned()),
                host: socket,
            }),
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }

    pub fn connect(&self) -> Result<String, Error> {
        // connect to the host
        Ok("Connected".to_owned())
    }

    pub fn send(&self, file: ProjectFile) -> Result<String, Error> {
        // send file to the host
        Ok("Sent".to_owned())
    }
}

impl FromStr for RenderNode {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<RenderNode>(s)
    }
}
