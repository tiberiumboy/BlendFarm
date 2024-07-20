use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, PartialOrd, Eq, Ord)]
pub struct BlenderData {
    pub executable: PathBuf,
    pub version: Version,
}
