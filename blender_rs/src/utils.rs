use std::{env::consts, path::PathBuf};

/// Return extension matching to the current operating system (Only display Windows(.zip), Linux(.tar.xz), or macos(.dmg)).
// Rely on providing valid extension to use. This seems backward.
pub(crate) fn get_extension() -> Result<String, String> {
    match consts::OS {
        "windows" => Ok(".zip".to_owned()),
        "macos" => Ok(".dmg".to_owned()),
        "linux" => Ok(".tar.xz".to_owned()),
        os => Err(os.to_string()),
    }
}

/// fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
pub(crate) fn get_valid_arch() -> Result<String, String> {
    match consts::ARCH {
        "x86_64" => Ok("x64".to_owned()),
        "aarch64" => Ok("arm64".to_owned()),
        arch => Err(arch.to_string()),
    }
}

/// Fetch the configuration path for blender. 
/// This is used to store temporary files and configuration files for blender.
/// TODO: Consider loading this from user preferences?
pub(crate) fn get_config_path() -> PathBuf {
    dirs::config_dir().unwrap().join("BlendFarm")
}

// TODO: this is ugly, and I want to get rid of this. How can I improve this?
// Backstory: Win and linux can be invoked via their direct app link. However, MacOS .app is just a bundle, which contains the executable inside.
// To run process::Command, I must properly reference the executable path inside the blender.app on MacOS, using the hardcoded path below.
pub(crate) const MACOS_PATH: &str = "Contents/MacOS/Blender";