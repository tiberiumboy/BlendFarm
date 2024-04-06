use crate::models::job::Job;
use std::io;
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

// pretty soon we will invoke this method!
#[allow(dead_code)]
pub fn render(job: &Job, frame: i32) -> io::Result<()> {
    let path = job
        .project_file
        .tmp
        .or_else(|| Some(job.project_file.src))
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap();

    let output = job.output.as_os_str().to_str().unwrap();

    command
        .args([
            "--factory-startup", // skip startup.blend
            "-noaudio",          // no sound
            "-b",                // background
            path,
            "-o", // output
            output,
            // --log "*" to log everything
            "-f", // frame (must be last!)
            &frame.to_string(),
        ])
        .output();

    Ok(())
}

// TODO: implement a method to download blender from source

// TOOD: fetch blender configurations?
