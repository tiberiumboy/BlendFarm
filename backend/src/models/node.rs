use message_io::network::Endpoint;

// dead code?
#[derive(Debug, Eq, PartialEq, Hash)]
pub(crate) struct Node {
    // may have to come up with some caveat for this
    pub name: String,
    pub endpoint: Endpoint,
}

// dead code?
impl Node {
    pub fn new(name: &str, endpoint: Endpoint) -> Self {
        Self {
            name: name.to_owned(),
            endpoint,
        }
    }
}
