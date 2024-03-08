pub struct ClientNode {
    pub id: String,
    #[serde(skip_serializing)]
    pub ip: String,
    #[serde(skip_serializing)]
    pub port: u16,
    pub name: Option<String>,
}

impl ClientNode {
    pub fn new(ip: String, port: u16) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            ip: ip.to_owned(),
            port: port,
            name: None,
        }
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }
}
