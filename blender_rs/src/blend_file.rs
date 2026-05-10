use std::{
    fs,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    blender::BlenderError,
    models::{
        config::BlenderConfiguration, peek_response::PeekResponse, render_setting::RenderSetting,
        scene_info::SceneInfo,
    },
    utils::get_config_path,
};

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
    fn get_script_path() -> PathBuf {
        get_config_path().join("render.py")
    }

    pub fn new(path_to_blend_file: impl AsRef<Path>) -> Result<Self, BlenderError> {
        let blend = Blend::from_path(&path_to_blend_file)
            // BUG: *BlendParseError contains different traits that's preventing me using anyhow error traits implementation.
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
            inner: path_to_blend_file.as_ref().to_path_buf(),
            major,
            minor,
            render_setting,
            scene_info,
        })
    }

    pub fn setup_args(&self, settings: &BlenderConfiguration) -> Result<Vec<String>, BlenderError> {
        let script_path = Self::get_script_path();
        if !script_path.exists() {
            let data = include_bytes!("./render.py");
            fs::write(&script_path, data).map_err(|e| BlenderError::PythonError(e.to_string()))?;
        }

        let path = self.to_path().as_os_str().to_os_string();
        // provide the configuration in json format
        let content = serde_json::to_string(settings)
            .map_err(|e| BlenderError::InvalidFile(e.to_string()))?;

        Ok(vec![
            "--factory-startup".to_owned(),
            "-noaudio".into(),
            "-b".into(),
            fs::canonicalize(path)
                .unwrap()
                .to_str()
                .unwrap_or_default()
                .to_owned(),
            "-P".into(),
            script_path.to_str().unwrap().into(),
            "--".into(),
            "-c".into(),
            // does this handle escaped characters?
            content,
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
    use super::*;
    use crate::models::config::tests::mock_blender_configuration;
    use crate::models::render_setting::tests::mock_rendering_setting;
    use crate::models::scene_info::tests::mock_scene_info;

    fn mock_blendfile() -> BlendFile {
        let scene_info = mock_scene_info();
        let render_setting = mock_rendering_setting();
        BlendFile {
            inner: PathBuf::new(),
            major: 4,
            minor: 2,
            scene_info,
            render_setting,
        }
    }

    #[test]
    fn test_setup_args() {
        // here we will test the argument and verify that this is the correct cli usage to call blender application.
        // In this method imoplementation it will verify that the python file exist before running blender application.
        // Ensure the python script exist at the end of the test.
        let mock_blend_file = mock_blendfile();
        let mock_blend_config = mock_blender_configuration();
        let args = mock_blend_file.setup_args(&mock_blend_config);
        assert!(args.is_ok());
        let script_path = BlendFile::get_script_path();
        assert!(script_path.exists())
    }
}
