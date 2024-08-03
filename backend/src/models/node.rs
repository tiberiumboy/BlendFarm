use message_io::network::Endpoint;

// TODO: I wonder if I should keep it strongly named like NodeStatus, or loosely named Status under node::Status respectively?
#[derive(Debug, PartialEq, Eq, Hash, Default)]
pub(crate) enum NodeStatus {
    Disconnected,
    #[default]
    Idle,
    Running,
    Paused,
    Completed,
    Error(String),
}

// TODO: I'm worry about name ambiguous here
#[derive(Debug, Eq, Hash)]
pub(crate) struct Node {
    pub name: String,
    pub endpoint: Endpoint,
    pub status: NodeStatus,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.endpoint == other.endpoint
    }
}

impl Node {
    pub fn new(name: &str, endpoint: Endpoint) -> Self {
        Self {
            name: name.to_string(),
            endpoint,
            status: NodeStatus::default(),
        }
    }
}
