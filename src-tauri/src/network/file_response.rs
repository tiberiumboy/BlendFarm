use crate::network::FileData;
use serde::{Deserialize, Serialize};

// Is this struct publicitized or localized? Prefer to make this private.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResponse(FileData);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_new_file_response_succeed() {
        let data: FileData = Vec::new();
        let response = FileResponse::new(data.clone());
        assert_eq!(response.0, data);
    }

    #[test]
    fn ensure_response_into_data_succeed() {
        let data: FileData = Vec::new();
        let response = FileResponse::new(data.clone());
        let result: FileData = response.into();
        assert_eq!(result, data);
    }
}
