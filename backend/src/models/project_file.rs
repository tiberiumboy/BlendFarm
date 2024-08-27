/*  DEV BLOG
    Had a brainfart thinking about this file for a second. I would like to know if it's possible to gather scene information by running
    python script on the blend file, and extract out whatever information necessary. ((plugins?Eevee/cycle?Cameras?Sample size?))
        - Consider this as a feature for future implementation - but now I need to ask tester for valuable information to extract from blender.

    I'm running into issue where sending this project file over to other network node server - doesn't recognize the source of the project file anymore.
    To fix this - I will need to extract the file name, and then send this information over.
    I will need to ask the server configuration to load user desire path for blender file location.
*/

use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, io, path::PathBuf, str::FromStr};

use super::server_setting::ServerSetting;

// TODO: this may ultimately get removed? We just need the pathbuf to the blender file specifically..
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectFile {
    file_name: String,
}

impl PartialEq for ProjectFile {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
}

pub fn get_project_collections() -> Vec<ProjectFile> {
    let server = ServerSetting::load();
    if !server.blend_dir.exists() {
        return Vec::new();
    }

    // validate and see if this doesn't break
    // need to find a way to filter reading dir to only *.blend extension.
    match server.blend_dir.read_dir() {
        Ok(entries) => {
            // let mut col = Vec::with_capacity(*(&entries.count()));
            let mut col = Vec::with_capacity(20); // temp fixes
            for entry in entries {
                if let Ok(dir_entity) = entry {
                    let file_path = dir_entity.path();
                    if file_path.is_file() && file_path.extension().unwrap().eq(OsStr::new("blend"))
                    {
                        let project_file = ProjectFile::new(file_path).unwrap();
                        col.push(project_file);
                    }
                }
            }

            col
        }
        Err(_) => Vec::new(),
    }
}

impl ProjectFile {
    pub fn new(src: PathBuf) -> Result<Self, io::Error> {
        // Here - we need to do two things-
        // one, we need to make a copy of this file, and put it in the blender working directory
        // then save the blender file name instead of the full path to the source.
        if let Some(file_name) = &src.file_name() {
            let server = ServerSetting::load();
            let mut dst = server.blend_dir;
            dst.push(&src.file_name().unwrap());
            let name = file_name.to_str().unwrap().to_string();

            if let Err(e) = std::fs::copy(&src, &dst) {
                println!("Unable to copy file from [{src:?}] to [{dst:?}]: {e}");
            }
            return Ok(Self { file_name: name });
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source is not a file type!",
            ))
        }
    }

    // I find this useful to return the path of the recent file location.
    pub(crate) fn file_path(&self) -> PathBuf {
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
