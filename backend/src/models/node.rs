use message_io::network::Endpoint;

// TODO: I'm worry about name ambiguous here
#[derive(Debug, Eq, PartialEq, Hash)]
pub(crate) struct Node {
    pub endpoint: Endpoint,
}

impl Node {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }
}
