use crate::models::error::Error;
use crate::services::sender;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, net::SocketAddr, path::PathBuf, str::FromStr};

#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
#[derive(Debug, Serialize, Deserialize)]
pub struct Running;
#[derive(Debug)]
pub struct Inactive;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode<State = Idle> {
    pub name: Option<String>,
    pub host: SocketAddr,
    state: PhantomData<State>,
}

#[allow(dead_code)]
impl RenderNode<Inactive> {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode<Inactive>, Error> {
        match host.parse::<SocketAddr>() {
            Ok(socket) => Ok(RenderNode {
                name: Some(name.to_owned()),
                state: PhantomData::<Inactive>,
                host: socket,
            }),
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }

    pub fn connect(self) -> RenderNode<Idle> {
        RenderNode {
            name: self.name,
            host: self.host,
            state: PhantomData::<Idle>,
        }
    }
}

impl RenderNode<Idle> {
    // TODO: Find a reason to keep this code around...?
    #[allow(dead_code)]
    pub fn create_localhost() -> Self {
        let host = SocketAddr::from_str("127.0.0.1:15000").unwrap();
        Self {
            name: Some("localhost".to_owned()),
            host,
            state: PhantomData::<Idle>,
        }
    }

    #[allow(dead_code)]
    pub fn disconnected(self) -> RenderNode<Inactive> {
        RenderNode {
            name: self.name,
            host: self.host,
            state: PhantomData::<Inactive>,
        }
    }

    #[allow(dead_code)]
    pub fn send(self, file: &PathBuf) {
        sender::send(file, &self);
        // RenderNode {
        //     id: self.id,
        //     name: self.name,
        //     host: self.host,
        //     state: std::marker::PhantomData::<Running>,
        // }
    }
}

#[allow(dead_code)]
impl RenderNode<Running> {
    pub fn abort(self) -> RenderNode<Idle> {
        RenderNode {
            name: self.name,
            host: self.host,
            state: std::marker::PhantomData::<Idle>,
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
