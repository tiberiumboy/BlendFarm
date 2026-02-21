use std::{
    fs,
    net::SocketAddrV4,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    blender::{BlenderError, Frame},
    models::{
        blender_scene::{BlenderScene, Camera, Sample, SceneName},
        engine::Engine,
        format::Format,
        peek_response::PeekResponse,
        render_setting::{FrameRate, RenderSetting},
        window::Window,
    },
    utils::get_config_path,
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SceneInfo {
    pub scenes: Vec<SceneName>,
    pub cameras: Vec<Camera>,
    pub frame_start: Frame,
    pub frame_end: Frame,
    render_width: i32,
    render_height: i32,
    fps: FrameRate,
    sample: Sample,
    output: PathBuf,
    engine: Engine,
}

impl SceneInfo {
    pub fn selected_camera(&self) -> String {
        self.cameras.get(0).unwrap_or(&"".to_owned()).to_owned()
    }

    pub fn selected_scene(&self) -> String {
        self.scenes.get(0).unwrap_or(&"".to_owned()).to_owned()
    }

    pub(crate) fn process(mut self, blend: &Blend) -> Result<Self, BlenderError> {
        // this denotes how many scene objects there are.
        for obj in blend.instances_with_code(*b"SC") {
            let scene = obj.get("id").get_string("name").replace("SC", ""); // not the correct name usage?
            let render = &obj.get("r"); // get render data

            // do need to make sure that the engine is correctly set?
            self.engine = match render.get_string("engine") {
                x if x.contains("NEXT") => Engine::BLENDER_EEVEE_NEXT,
                x if x.contains("EEVEE") => Engine::BLENDER_EEVEE,
                x if x.contains("OPTIX") => Engine::OPTIX,
                _ => Engine::CYCLES,
            };

            self.sample = obj.get("eevee").get_i32("taa_render_samples");

            // Issue, Cannot find cycles info! Blender show that it should be here under SCscene, just like eevee, but I'm looking it over and over and it's not there? Where is cycle?
            // Use this for development only!
            // Self::explore_value(&obj.get("eevee"));

            self.render_width = render.get_i32("xsch");
            self.render_height = render.get_i32("ysch");
            self.frame_start = render.get_i32("sfra");
            self.frame_end = render.get_i32("efra");
            self.fps = render.get_u16("frs_sec");
            self.output = render
                .get_string("pic")
                .parse::<PathBuf>()
                .map_err(|e| BlenderError::PythonError(e.to_string()))?;

            self.scenes.push(scene);
        }

        // interesting - I'm picking up the wrong camera here?
        for obj in blend.instances_with_code(*b"CA") {
            let camera = obj.get("id").get_string("name").replace("CA", "");
            self.cameras.push(camera);
        }

        Ok(self)
    }

    pub fn render_setting(self) -> RenderSetting {
        RenderSetting::new(
            self.output,
            self.render_width,
            self.render_height,
            self.sample,
            self.fps,
            self.engine,
            Format::default(),
            Window::default(),
        )
    }

    pub(crate) fn peek_response(&self, version: &Version) -> PeekResponse {
        let selected_scene = self.selected_scene();
        let selected_camera = self.selected_camera();

        let render_setting: RenderSetting = self.clone().render_setting();
        let current = BlenderScene::new(selected_scene, selected_camera, render_setting);

        PeekResponse::new(
            version.clone(),
            self.frame_start,
            self.frame_end,
            self.cameras.clone(),
            self.scenes.clone(),
            current,
        )
    }
}

// A struct to hold valid blend file with compatible partial version.
// we can extract additional data if we need to?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlendFile {
    inner: PathBuf,
    major: u16,
    minor: u16,
    scene_info: SceneInfo,
    render_setting: RenderSetting,
}

impl BlendFile {
    pub fn new(path_to_blend_file: &Path) -> Result<Self, BlenderError> {
        let blend = Blend::from_path(&path_to_blend_file)
            // TODO: try to handle BlendParseError? Future work
            .map_err(|e| {
                BlenderError::InvalidFile(format!("Received BlenderParseError! {e:?}").to_owned())
            })?;

        // blender version are display as three digits number, e.g. 404 is major: 4, minor: 4.
        // treat this as a u16 major = u16 / 100, minor = u16 % 100;
        let str_version = std::str::from_utf8(&blend.blend.header.version)
            .map_err(|e| BlenderError::InvalidFile(e.to_string()))?;

        let value: u16 = str_version
            .parse()
            .map_err(|e: ParseIntError| BlenderError::InvalidFile(e.to_string()))?;
        let major = value / 100;
        let minor = value % 100;

        let scene_info = SceneInfo::default().process(&blend)?;
        let render_setting = scene_info.clone().render_setting();

        Ok(BlendFile {
            inner: path_to_blend_file.to_path_buf(),
            major,
            minor,
            render_setting,
            scene_info,
        })
    }

    pub fn setup_args(&self, socket: &SocketAddrV4) -> Result<Vec<String>, BlenderError> {
        let script_path = get_config_path().join("render.py");
        if !script_path.exists() {
            let data = include_bytes!("./render.py");
            fs::write(&script_path, data).map_err(|e| BlenderError::PythonError(e.to_string()))?;
        }

        let path = self.to_path().as_os_str().to_os_string();

        Ok(vec![
            // "--factory-startup".to_owned(),
            // "-noaudio".into(),
            "-b".into(),
            path.to_str().unwrap().to_owned(),
            "-P".into(),
            script_path.to_str().unwrap().into(),
            "--".into(),
            "-i".into(),
            socket.ip().to_string(),
            "-p".into(),
            socket.port().to_string(),
        ])
    }

    pub fn get_partial_version(&self) -> (u16, u16) {
        (self.major, self.minor)
    }

    pub fn peek_response(&self, version: Option<&Version>) -> PeekResponse {
        let last_version = match version {
            Some(v) => v,
            None => &Version::new(self.major.into(), self.minor.into(), 0),
        };
        self.scene_info.peek_response(last_version)
    }

    pub fn to_path(&self) -> &Path {
        self.inner.as_path()
    }
}

impl Into<PathBuf> for BlendFile {
    fn into(self) -> PathBuf {
        self.inner
    }
}

impl Into<RenderSetting> for BlendFile {
    fn into(self) -> RenderSetting {
        self.render_setting
    }
}

impl Into<SceneInfo> for BlendFile {
    fn into(self) -> SceneInfo {
        self.scene_info
    }
}

#[cfg(test)]
mod tests {
    // use crate::blend_file::BlendFile;

    // fn mock_blendfile() -> BlendFile {
    //     let blend_file = BlendFile::new(path_to_blend_file)
    // }
}
