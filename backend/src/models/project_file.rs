/*  DEV BLOG
    Had a brainfart thinking about this file for a second. I would like to know if it's possible to gather scene information by running
    python script on the blend file, and extract out whatever information necessary. ((plugins?Eevee/cycle?Cameras?Sample size?))
        - Consider this as a feature for future implementation - but now I need to ask tester for valuable information to extract from blender.

    I'm running into issue where sending this project file over to other network node server - doesn't recognize the source of the project file anymore.
    To fix this - I will need to extract the file name, and then send this information over.
    I will need to ask the server configuration to load user desire path for blender file location.
*/

use serde::{Deserialize, Serialize};
use std::{
    io::{self, ErrorKind},
    path::PathBuf,
    str::FromStr,
};

use super::server_setting::ServerSetting;

// TODO: this may ultimately get removed? We just need the pathbuf to the blender file specifically..
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProjectFile {
    file_name: String,
}

impl ProjectFile {
    pub fn new(src: PathBuf) -> Result<Self, io::Error> {
        // enforce it so that we are only going to accept .blend file extension (or any number after the extension?)
        if let Some(ext) = src.extension() {
            if ext == "blend" {
                let file_name = src.file_name().unwrap();
                return Ok(Self {
                    file_name: file_name.to_str().unwrap().to_string(),
                });
            }
        }

        Err(io::Error::new(
            ErrorKind::InvalidInput,
            "Must be a blender file extension!",
        ))
    }

    // I find this useful to return the path of the recent file location.
    pub(crate) fn file_path(&self) -> PathBuf {
        // and another problem here...
        let server = ServerSetting::load();
        let path = server.blend_dir.to_owned();
        path.join(&self.file_name)
    }
}

impl FromStr for ProjectFile {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: ProjectFile = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}
