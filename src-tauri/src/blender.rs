use std::{
    io,
    path::Path,
    process::{Command, Output},
};

enum Device {
    CPU,
    CUDA,
    OPTIX,
    HIP,
    ONEAPI,
    METAL,
}
// append +CPU to gpu to include CPU into render cycle.

// how can I detect what device is allowed on this current machine farm?

#[derive(Default)]
pub struct Blender {}

impl Blender {
    pub fn render(
        &self,
        path: impl AsRef<Path>,
        output: impl AsRef<Path>,
        frame: i32,
    ) -> io::Result<Output> {
        let frame = frame.to_string();
        // in the original program, the command used to launch blender doesn't invoke to render - it execute a python script using -P "script.py" with argument passed to python instead.
        // see "render.py" for the python script used in blendfarm - ignored by repo
        Command::new("blender")
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
}
