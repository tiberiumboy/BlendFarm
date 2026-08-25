use serde::{Deserialize, Serialize};

// Simple file exchange protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileRequest(String);

impl FileRequest {
    pub fn new(name: String) -> Self {
        FileRequest(name)
    }
}

impl Into<String> for FileRequest {
    fn into(self) -> String {
        self.0.to_owned()
    }
}