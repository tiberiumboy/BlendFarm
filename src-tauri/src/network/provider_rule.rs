use crate::network::message::KeywordSearch;
use std::{/* ffi::OsStr,*/ path::PathBuf};

pub enum ProviderRule {
    // Use "file name.ext", Extracted from PathBuf.
    Default(PathBuf),
    // Custom keyword search for specific PathBuf.
    Custom(KeywordSearch, PathBuf),
}

// impl ProviderRule {
//     pub fn get_file_name(&self) -> Option<&OsStr> {
//         match self {
//             ProviderRule::Default(path) => path.file_name(),
//             ProviderRule::Custom(_, path_buf) => path_buf.file_name(),
//         }
//     }
// }
