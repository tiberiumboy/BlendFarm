use super::render_setting::RenderSetting;
use serde::{Deserialize, Serialize};

pub type SceneName = String;
pub type Camera = String;
pub type Sample = i32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlenderScene {
    /// Name of the scene
    pub scene: SceneName,
    /// Camera reference name to render from
    pub camera: Camera,
    /// Render Settings
    pub render_setting: RenderSetting,
}

impl BlenderScene {
    pub fn new(scene: SceneName, camera: Camera, render_setting: RenderSetting) -> Self {
        Self {
            scene,
            camera,
            render_setting,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::models::render_setting::tests::mock_rendering_setting;

    pub fn mock_blender_scene() -> BlenderScene {
        // TODO: why do I have another render settings that can be different than the other render settiings?
        let render_setting = mock_rendering_setting();
        BlenderScene {
            scene: "Test".to_owned(),
            camera: "Camera01".to_owned(),
            render_setting,
        }
    }
}
