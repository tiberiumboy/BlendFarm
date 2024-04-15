use crate::models::error::Error;
use crate::services::sender;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, net::SocketAddr, path::PathBuf, str::FromStr};

#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
#[derive(Debug, Serialize, Deserialize)]
pub struct Running;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode<State = Idle> {
    pub id: String,
    pub name: Option<String>,
    pub host: SocketAddr,
    pub os: String,
    state: PhantomData<State>,
}

impl RenderNode<Idle> {
    pub fn connect(self) -> RenderNode<Idle> {
        RenderNode {
            id: self.id,
            name: self.name,
            host: self.host,
            os: self.os,
            state: PhantomData::<Idle>,
        }
    }

    #[allow(dead_code)]
    pub fn send(self, file: &PathBuf) -> RenderNode<Running> {
        sender::send(file, &self);
        RenderNode {
            id: self.id,
            name: self.name,
            host: self.host,
            os: self.os,
            state: PhantomData::<Running>,
        }
    }
}

impl RenderNode<Running> {
    pub fn abort(self) -> RenderNode<Idle> {
        RenderNode {
            id: self.id,
            name: self.name,
            host: self.host,
            os: self.os,
            state: PhantomData::<Idle>,
        }
    }
}

impl RenderNode {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode<Idle>, Error> {
        match host.parse::<SocketAddr>() {
            Ok(socket) => Ok(RenderNode {
                // connect to host, and retrieve their system info:
                // get their OS as well
                id: uuid::Uuid::new_v4().to_string(),
                name: Some(name.to_owned()),
                state: PhantomData::<Idle>,
                os: "unknown".to_owned(),
                host: socket,
            }),
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }
}

impl FromStr for RenderNode {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<RenderNode>(s)
    }
}
