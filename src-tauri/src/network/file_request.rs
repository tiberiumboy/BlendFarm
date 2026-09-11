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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_new_file_request_succeed() {
        let name = "test".to_owned();
        let request = FileRequest::new(name.clone());
        assert_eq!(request.0, name);
    }

    #[test]
    fn ensure_file_request_into_name_succeed() {
        let name = "test".to_owned();
        let request = FileRequest::new(name.clone());
        let result: String = request.into();
        assert_eq!(result, name);
    }
}
