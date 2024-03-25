use chrono::prelude::*;
// use serde::{Deserializer, Serializer};
use std::{env, os};

// find a way to obtain the operating system information
// static string[] REQUIRED_OS
const OS_LINUX64: &str = "linux64";
const OS_WINDOWS64: &str = "windows64";
const OS_MACOS: &str = "macOS";

const VERSIONS_URL: &str = "https://download.blender.org/release/";

const CACHE_DAYS: u8 = 3;

pub struct BlenderVersion {
    is_custom: bool,
    name: String,
    utc: DateTime<Utc>,
    url: String,
}

impl Default for BlenderVersion {
    fn default() -> Self {
        Self {
            is_custom: false,
            name: "Blender".to_owned(),
            utc: Utc::now(),
            url: "localhost".to_owned(),
        }
    }
}

impl BlenderVersion {
    fn download(&self) {
        let os = env::consts::OS;
        let ext = "tar.xy";
        // todo - correct arch labeling, e.g. x86_64 -> x64, arm -> arm64, etc
        let arch = env::consts::ARCH;
        let archive = "blender-{}-{os}-{arch}.{ext}";
    }
}
