use message_io::network::Endpoint;

// TODO: I'm worry about name ambiguous here
#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq, Hash)]
pub(crate) struct Node {
    // may have to come up with some caveat for this
    pub name: String,
    pub endpoint: Endpoint,
}

impl Node {
    #[allow(dead_code)]
    pub fn new(name: &str, endpoint: Endpoint) -> Self {
        Self {
            name: name.to_owned(),
            endpoint,
        }
    }
}
