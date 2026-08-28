use crate::network::FileData;
use serde::{Deserialize, Serialize};

// Is this struct publicitized or localized? Prefer to make this private.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileResponse(FileData);

impl FileResponse {
    pub fn new(data: FileData) -> Self {
        FileResponse(data)
    }
}

impl Into<FileData> for FileResponse {
    fn into(self) -> FileData {
        self.0
    }
}
