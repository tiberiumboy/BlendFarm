use message_io::network::Endpoint;

// TODO: I'm worry about name ambiguous here
#[derive(Debug, Eq, Hash)]
pub(crate) struct Node {
    pub endpoint: Endpoint,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.endpoint == other.endpoint
    }
}

impl Node {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }
}
