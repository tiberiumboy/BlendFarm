use std::path::PathBuf;

use blender::blender::Blender;
use futures::channel::mpsc::Sender;
use semver::Version;

use crate::services::tauri_app::{BlenderQuery, QueryMode};

#[derive(Debug)]
pub enum BlenderAction {
    Add(PathBuf),
    List(Sender<Option<Vec<BlenderQuery>>>, QueryMode),
    Get(Version, Sender<Option<Blender>>),
    Disconnect(Blender), // detach links associated with file path, but does not delete local installation!
    Remove(Blender), // deletes local installation of blender, use it as last resort option. (E.g. force cache clear/reinstall/ corrupted copy)
}

impl PartialEq for BlenderAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Add(l0), Self::Add(r0)) => l0 == r0,
            (Self::List(.., l0), Self::List(.., r0)) => l0 == r0,
            (Self::Get(l0, ..), Self::Get(r0, ..)) => l0 == r0,
            (Self::Disconnect(l0), Self::Disconnect(r0)) => l0 == r0,
            (Self::Remove(l0), Self::Remove(r0)) => l0 == r0,
            _ => false,
        }
    }
}
