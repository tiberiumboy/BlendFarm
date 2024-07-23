use crate::models::{device::Device, engine::Engine, format::Format, mode::Mode};
use serde::{Deserialize, Serialize};
use std::{
    env::temp_dir,
    fs,
    path::{Path, PathBuf},
};

/*
    Developer blog

    - Having done extensive research, Blender have two ways to interface to the program
        1. Through CLI
        2. Through Python API via "bpy" library

    Review online for possible solution to interface blender via CAPI, but was strongly suggested to use a python script instead
    this limits what I can do in term of functionality, but it'll be a good start.
    FEATURE - See if python allows pointers/buffer access to obtain job render progress - Allows node to send host progress result. Possibly viewport network rendering?

    BlendFarm code shows that they heavily rely on using python code to perform exact operation.
    Question is, do I want to use their code, or do I want to stick with CLI instead?
    I'll try implement both solution, CLI for version and other basic commands, python for advance features and upgrade?
*/

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Args {
    pub file: PathBuf,          // required
    pub output: PathBuf,        // optional
    pub mode: Mode,             // required
    pub engine: Option<Engine>, // optional
    pub device: Option<Device>, // optional
    pub format: Option<Format>, // optional - default to Png
    pub use_continuation: bool, // optional - default to false
}

impl Args {
    pub fn new(file: impl AsRef<Path>, output: impl AsRef<Path>, mode: Mode) -> Self {
        Args {
            file: file.as_ref().to_path_buf(),
            output: output.as_ref().to_path_buf(),
            mode,
            engine: None,
            device: None,
            format: None,
            use_continuation: false,
        }
    }

    pub fn create_arg_list(&self, json_path: PathBuf) -> Vec<String> {
        // so - in this code - the guy created a temp json file and have python load that json file instead.
        // string arg = $"--factory-startup -noaudio -b \"{Path.GetFullPath(file)}\" -P \"{GetRenderScriptPath()}\" -- \"{path}\" {USE_CONTINUATION}";z
        let script_path = temp_dir().join("render.py");
        if !script_path.exists() {
            let data = include_bytes!("../render.py");
            fs::write(&script_path, data).unwrap();
        }

        vec![
            "--factory-startup".to_owned(),
            "-noaudio".to_owned(),
            "-b".to_owned(),
            self.file.to_str().unwrap().to_string(),
            "-P".to_owned(),
            script_path.to_str().unwrap().to_owned(),
            "--".to_owned(),
            json_path.to_str().unwrap().to_string(),
            format!("{}", self.use_continuation),
        ]
        /*

        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        // # is substitute to 0 pad, none will add to suffix four pounds (####)
        let mut col = vec![
            "-b".to_owned(),
            self.file.to_str().unwrap().to_string(),
            "-o".to_owned(),
            self.output.to_str().unwrap().to_string(),
        ];

        // col.push("-E".to_owned());
        // col.push(self.engine.to_string());
        // col.push("F".to_owned());
        // col.push(self.format.to_string());
        // col.push("-X".to_owned());

        if let Some(engine) = &self.engine {
            col.push("-E".to_owned());
            col.push(engine.to_string());
        }
        if let Some(format) = &self.format {
            col.push("-F".to_owned());
            col.push(format.to_string());
            col.push("-X".to_owned()); // explicitly use extension
        }

        // this argument must be set at the very end
        let mut additional_args = match self.mode {
            Mode::Frame(frame) => {
                // could there be a better way to do this?
                vec!["-f".to_owned(), frame.to_string()]
            }
            // Render the whole animation using all the settings saved in the blend-file.
            Mode::Animation { start, end } => vec![
                "-s".to_owned(),
                start.to_string(),
                "-e".to_owned(),
                end.to_string(),
                "-a".to_owned(),
            ],
            Mode::None => vec![],
        };

        col.append(&mut additional_args);

        // Cycles add-on options must be specified following a double dash.

        // if self.engine == Engine::Cycles {
        //     col.push("-- --cycles-device".to_owned());
        //     col.push(self.device.to_string());
        // }
        if Some(Engine::Cycles) == self.engine {
            if let Some(device) = &self.device {
                col.push("-- --cycles-device".to_owned());
                col.push(device.to_string());
            }
        }

        col
        */
    }
}