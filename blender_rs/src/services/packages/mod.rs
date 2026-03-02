use crate::blender::Blender;

pub(crate) mod custom;
pub(crate) mod download_link;
pub(crate) mod downloaded;
pub(crate) mod bundle;
pub(crate) mod package;

pub(crate) trait BlenderPath {
    fn get_blender(&self) -> Option<Blender>;
}