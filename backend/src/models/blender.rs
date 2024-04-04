use std::{
    io,
    path::Path,
    process::{Command, Output},
};

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

#[derive(Default)]
pub struct Blender {}

// maybe?
// const OS_LINUX64: &str = "linux64";
// const OS_WINDOWS64: &str = "window64";
// const OS_MACOS: &str = "macOS";
// const OS_MACOSARM64: &str = "macOS-arm64";

impl Blender {
    // pretty soon we will invoke this method!
    #[allow(dead_code)]
    pub fn render(
        &self,
        path: impl AsRef<Path>,
        output: impl AsRef<Path>,
        frame: i32,
    ) -> io::Result<Output> {
        let frame = frame.to_string();

        // in the original program, the command used to launch blender doesn't invoke to render - it execute a python script using -P "script.py" with argument passed to python instead.
        // see "render.py" for the python script used in blendfarm - ignored by repo
        // who knew, macOS is special!
        let mut command = if cfg!(target_os = "macos") {
            Command::new("/Applications/Blender.app/Contents/MacOS/blender")
        } else {
            Command::new("blender")
        };

        command
            .args([
                "--factory-startup", // skip startup.blend
                "-noaudio",          // no sound
                "-b",                // background
                path.as_ref().to_str().expect("Missing path!"),
                "-o", // output
                output
                    .as_ref()
                    .to_str()
                    .expect("Missing output destination!"),
                // --log "*" to log everything
                "-f", // frame (must be last!)
                &frame,
            ])
            .output()
    }

    // TODO: implement a method to download blender from source

    // TOOD: fetch blender configurations?
}
