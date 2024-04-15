// use crate::blender::version::Blender;
use crate::services::{blender::Blender, sender::send};
// use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf, str::FromStr};
use uuid::Uuid;

use super::render_node::RenderNode;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub id: String,
    pub file_name: String,
    pub src: PathBuf,
    #[serde(skip_serializing)]
    pub tmp: Option<PathBuf>,
    pub blender_version: Blender,
}

#[allow(dead_code)]
impl ProjectFile {
    pub fn new(path: &PathBuf) -> Self {
        //let blend = Blend::from_path(path).expect("Unable to read blend file!");
        let version = Version::new(2, 93, 0); //blend.version().unwrap();
        Self {
            id: Uuid::new_v4().to_string(),
            // TODO: Clean this up afterward!
            file_name: path.file_name().unwrap().to_str().unwrap().to_owned(),
            src: path.to_owned(),
            tmp: None,
            blender_version: Blender::from_version(version),
        }
    }

    pub fn parse(src: &str) -> Result<ProjectFile, std::io::Error> {
        let path = PathBuf::from(src);
        Ok(ProjectFile::new(&path))
    }

    pub(crate) fn move_to_temp(&mut self) {
        let mut dir = env::temp_dir();
        let file_name = self.src.file_name().unwrap();
        dir.push(file_name);
        let _ = std::fs::copy(&self.src, &dir);
        self.tmp = Some(dir);
    }

    pub(crate) fn clear_temp(&mut self) {
        if let Some(tmp) = &self.tmp {
            let _ = std::fs::remove_file(tmp);
        }
        self.tmp = None;
    }

    pub(crate) fn upload(&self, render_nodes: Vec<RenderNode>) {
        for node in render_nodes {
            // send file to node
            send(&self.src, &node);
        }
    }

    pub(crate) fn file_path(&self) -> &PathBuf {
        self.tmp.as_ref().unwrap_or(&self.src)
    }

    //#[allow(dead_code)]
    //pub fn run(&mut self, frame: i32) {
    // self.move_to_temp();
    // let blender = Blender::default();

    // let _output = blender.render(&self, frame).unwrap();
    // self.clear_temp();
    //}
}

impl FromStr for ProjectFile {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let obj: ProjectFile = serde_json::from_str(s).unwrap();
        Ok(obj)
    }
}

impl PartialEq for ProjectFile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
