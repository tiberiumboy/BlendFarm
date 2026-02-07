use serde::{Deserialize, Serialize};
use super::render_setting::RenderSetting;

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
    pub fn new(
        scene: SceneName,
        camera: Camera,
        render_setting: RenderSetting,
    ) -> Self {
        Self {
            scene,
            camera,
            render_setting,
        }
    }
}