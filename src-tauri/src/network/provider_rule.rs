// use crate::network::message::KeywordSearch;
use std::path::PathBuf;

// TODO: May not be needed?
pub enum ProviderRule {
    // Use "file name.ext", Extracted from PathBuf.
    Default(PathBuf),
    // Custom keyword search for specific PathBuf.
    // Custom(KeywordSearch),
}
