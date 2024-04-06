use crate::models::blender_version::BlenderVersion;
use crate::models::project_file::ProjectFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{io, process::Command};
use tauri::command;

// TODO: Once I figure out about getting blender configuration from the hardware, use this to return back to the host about this machine configuration
// enum Device {
//     CPU,
//     CUDA,
//     OPTIX,
//     HIP,
//     ONEAPI,
//     METAL,
// }
// append +CPU to gpu to include CPU into render cycle.

// how can I detect what device is allowed on this current machine farm?

// const OS_LINUX64: &str = "linux64";
// const OS_WINDOWS64: &str = "window64";
// const OS_MACOS: &str = "macOS";
// const OS_MACOSARM64: &str = "macOS-arm64";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Blender {
    version: BlenderVersion, // version of blender
    path: PathBuf,           // path to blender executable
}

// pretty soon we will invoke this method!
pub fn render(project: &ProjectFile, frame: i32) -> io::Result<()> {
    let path = project
        .tmp
        .unwrap_or_else(|| project.src)
        .map(PathBuf::into_os_string)
        .and_then(|p| p.into_string().ok());

    let output = job.output.as_os_str().to_str().unwrap();

    /*
    "--factory-startup", // skip startup.blend
    "-noaudio",          // no sound
    "-b",                // background
    path,
    "-o", // output
    output,
    // --log "*" to log everything
    "-f", // frame (must be last!)

    */
    let cmd = format!(
        "--factory-startup -noaudio -b {} -o {} -f {}",
        path, output, frame
    );

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(cmd.split(" "))
            .output()
            .expect("Fail to launch blender!")
    } else {
        Command::new("sh")
            .args(["-c", cmd.as_str()])
            .output()
            .expect("Fail to launch blender!")
    };

    Ok(())
}

// TODO: implement a method to download blender from source

// TOOD: fetch blender configurations?
