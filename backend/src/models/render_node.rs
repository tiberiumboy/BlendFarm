use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub id: String,
    #[serde(skip_serializing)]
    pub host: IpAddr,
    pub name: Option<String>,
}

impl RenderNode {
    pub fn parse(name: String, host: String) -> Result<RenderNode> {
        let ip = host.parse::<IpAddr>()?;
        Ok(RenderNode {
            id: uuid::Uuid::new_v4().to_string(),
            host: ip,
            name: Some(name),
        })
    }

    pub fn connect(&self) -> Result<String> {
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
