use crate::blender::BlenderConfig;
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
use std::env::consts::ARCH;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
use std::env::consts::OS;
use std::{
    fs,
    io::{Error as IoError, ErrorKind, Result as IoResult},
};
use std::{path::PathBuf, sync::OnceLock};

static EXT: OnceLock<String> = OnceLock::new();
static ARCH: OnceLock<String> = OnceLock::new();

// I want this utilities to be only available under feature request.
// This util requires additional library support to provide exact unified blender config location.
pub fn get_blend_config_from_local() -> IoResult<BlenderConfig> {
    let config_path = dirs::config_dir()
        .ok_or(IoError::new(
            ErrorKind::NotFound,
            "Unable to find config directory!".to_owned(),
        ))?
        .join("BlendFarm")
        .join("BlenderManager.json");
    let data = fs::read(config_path)?;
    Ok(serde_json::from_slice::<BlenderConfig>(&data)?)
}

/// Return extension matching to the current operating system. Windows(zip), Linux(tar.xz), or MacOS(dmg)
/// This will return extension name without the initial period. Any period is treated as extension of extension (e.g. tar.xz)
#[inline]
pub(crate) fn get_extension() -> Result<&'static str, &'static str> {
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return Err(OS);
    Ok(&EXT.get_or_init(|| {
        #[cfg(target_os = "windows")]
        return "zip".to_owned();
        #[cfg(target_os = "macos")]
        return "dmg".to_owned();
        #[cfg(target_os = "linux")]
        return "tar.xz".to_owned();
    }))
}

/// Fetch Valid architecture. "x64" or "arm64"(apple silicon)
#[inline]
pub(crate) fn get_valid_arch() -> Result<&'static str, &'static str> {
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    return Err(ARCH);
    Ok(&ARCH.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        return "x64".to_owned();
        #[cfg(target_arch = "aarch64")]
        return "arm64".to_owned();
    }))
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
#[cfg(target_os = "macos")]
pub(crate) const MACOS_PATH: &str = "Contents/MacOS/Blender";
