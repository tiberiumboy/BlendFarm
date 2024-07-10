use message_io::network::Endpoint;

// TODO: I'm worry about name ambiguous here
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct Node {
    pub name: String,
    pub endpoint: Endpoint,
}

impl Node {
    pub fn new(name: &str, endpoint: Endpoint) -> Self {
        Self {
            name: name.to_string(),
            endpoint,
        }
    }
}
