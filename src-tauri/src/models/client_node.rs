pub struct ClientNode {
    pub ip: String,
    pub port: u16,
    pub name: Option<String>,
}

impl ClientNode {
    pub fn new(ip: String, port: u16) -> Self {
        Self {
            ip: ip.to_owned(),
            port: port,
            name: None,
        }
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }
}
