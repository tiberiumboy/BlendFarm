use chrono::prelude::*;
use chrono::Duration;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, os};

// find a way to obtain the operating system information
// static string[] REQUIRED_OS
const OS_LINUX64: &str = "linux64";
const OS_WINDOWS64: &str = "windows64";
const OS_MACOS: &str = "macOS";

const VERSIONS_URL: &str = "https://download.blender.org/release/";

const CACHE_DAYS: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlenderVersion {
    url: String,      // URL to download
    path: PathBuf,    // path to the blender executable
    version: Version, // version of blender
}

// impl Default for BlenderVersion {
//     fn default() -> Self {
//         Self {
//             name: "Blender".to_owned(),
//             path: PathBuf,
//             version: Version,
//             utc: Utc::now(),
//         }
//     }
// }

impl BlenderVersion {
    fn download(&self) {
        let os = env::consts::OS;
        let ext = "tar.xy";
        // todo - correct arch labeling, e.g. x86_64 -> x64, arm -> arm64, etc
        let arch = env::consts::ARCH;
        let archive = format!("blender-{}-{os}-{arch}.{ext}", self.version);
    }
}
