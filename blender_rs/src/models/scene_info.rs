use std::path::PathBuf;

use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    blender::{BlenderError, Frame},
    models::{
        blender_scene::{BlenderScene, Camera, Sample, SceneName},
        format::Format,
        peek_response::PeekResponse,
        render_setting::{FrameRate, RenderSetting},
        window::Window,
    },
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
            // self.engine = match render.get_string("engine") {
            //     x if x.contains("NEXT") => Engine::BLENDER_EEVEE_NEXT,
            //     x if x.contains("EEVEE") => Engine::BLENDER_EEVEE,
            //     x if x.contains("OPTIX") => Engine::OPTIX,
            //     _ => Engine::CYCLES,
            // };

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

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn mock_scene_info() -> SceneInfo {
        SceneInfo {
            scenes: Vec::new(),
            cameras: Vec::new(),
            frame_start: 1,
            frame_end: 2,
            render_width: 1280,
            render_height: 720,
            fps: 20,
            sample: 100,
            output: PathBuf::new(),
        }
    }
}
