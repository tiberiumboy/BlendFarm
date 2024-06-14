use crate::models::error::Error;
use crate::services::sender;
use message_io::node::{NodeHandler, NodeListener};
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, net::SocketAddr, path::PathBuf, str::FromStr, thread};

#[derive(Debug, Serialize, Deserialize)]
pub struct Idle;
#[derive(Debug, Serialize, Deserialize)]
pub struct Running;
#[derive(Debug)]
pub struct Inactive;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderNode {
    pub name: String,
    pub host: SocketAddr,
}

#[allow(dead_code)]
impl RenderNode {
    pub fn parse(name: &str, host: &str) -> Result<RenderNode, Error> {
        match host.parse::<SocketAddr>() {
            Ok(socket) => Ok(RenderNode {
                name: name.to_owned(),
                host: socket,
            }),
            Err(e) => Err(Error::PoisonError(e.to_string())),
        }
    }

    pub fn connect(self) -> RenderNode {
        // TODO: find out how we can establish connection here?

        RenderNode {
            name: self.name,
            host: self.host,
        }
    }

    // TODO: Find a reason to keep this code around...?
    #[allow(dead_code)]
    pub fn create_localhost() -> Self {
        let host = SocketAddr::from_str("127.0.0.1:15000").unwrap();
        Self {
            name: "localhost".to_owned(),
            host,
        }
    }

    #[allow(dead_code)]
    pub fn disconnected(self) {}

    #[allow(dead_code)]
    pub fn send(self, file: &PathBuf) {
        sender::send(file, &self);
    }

    /// Invoke the render node to start running the job
    pub fn run(self) {
        // is this where we can set the jobhandler?
        // let handler = thread::spawn(|| {});
    }

    pub fn abort(self) {}
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
