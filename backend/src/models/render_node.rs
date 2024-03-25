use std::str::FromStr;

use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RenderNode {
    pub id: String,
    // this should be private
    #[serde(skip_serializing)]
    pub ip: String,
    // this should be private
    #[serde(skip_serializing)]
    pub port: u16,
    pub name: Option<String>,
}

impl RenderNode {
    pub(crate) fn new(ip: &str, port: u16) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ip: ip.to_owned(),
            port,
            name: None,
        }
    }
}

impl Default for RenderNode {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ip: local_ip().unwrap().to_string(),
            port: 15000,
            name: Some("localhost".to_owned()),
        }
    }
}

impl FromStr for RenderNode {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: RenderNode = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
