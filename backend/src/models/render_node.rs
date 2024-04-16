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
    pub id: String,
    pub name: Option<String>,
    pub host: SocketAddr,
    state: PhantomData<State>,
}

#[allow(dead_code)]
impl RenderNode<Inactive> {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode<Inactive>, Error> {
        match host.parse::<SocketAddr>() {
            Ok(socket) => Ok(RenderNode {
                id: uuid::Uuid::new_v4().to_string(),
                name: Some(name.to_owned()),
                state: std::marker::PhantomData::<Inactive>,
                host: socket,
            }),
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }

    pub fn connect(self) -> RenderNode<Idle> {
        RenderNode {
            id: self.id,
            name: self.name,
            host: self.host,
            state: std::marker::PhantomData::<Idle>,
        }
    }
}

impl RenderNode<Idle> {
    #[allow(dead_code)]
    pub fn disconnected(self) -> RenderNode<Inactive> {
        RenderNode {
            id: self.id,
            name: self.name,
            host: self.host,
            state: std::marker::PhantomData::<Inactive>,
        }
    }

    #[allow(dead_code)]
    pub fn send(self, file: &PathBuf) -> RenderNode<Running> {
        sender::send(file, &self);
        RenderNode {
            id: self.id,
            name: self.name,
            host: self.host,
            state: std::marker::PhantomData::<Running>,
        }
    }
}

#[allow(dead_code)]
impl RenderNode<Running> {
    pub fn abort(self) -> RenderNode<Idle> {
        RenderNode {
            id: self.id,
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
