use serde::{Deserialize, Serialize};

// Is this struct publicitized or localized? Prefer to make this private and use FileData instead?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileResponse(Vec<u8>);

impl FileResponse {
    pub fn new(data: Vec<u8>) -> Self {
        FileResponse(data)
    }
}

impl Into<Vec<u8>> for FileResponse {
    fn into(self) -> Vec<u8> {
        self.0
    }
}