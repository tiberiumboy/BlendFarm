use crate::device::Device;
use crate::engine::Engine;
use crate::format::Format;
use crate::mode::Mode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug, Serialize, Deserialize)]
pub struct Args {
    file: PathBuf,          // required
    output: PathBuf,        // optional
    mode: Mode,             // required
    engine: Option<Engine>, // optional
    device: Option<Device>, // optional
    format: Option<Format>, // optional
}

impl Args {
    pub fn new(file: PathBuf, output: PathBuf, mode: Mode) -> Self {
        Args {
            file,
            output,
            mode,
            engine: None,
            device: None,
            format: None,
        }
    }

    pub fn create_arg_list(&self) -> Vec<String> {
        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        // # is substitute to 0 pad, none will add to suffix four pounds (####)

        let mut col = vec![
            "-b".to_owned(),
            self.file.to_str().unwrap().to_string(),
            "-o".to_owned(),
            self.output.to_str().unwrap().to_string(),
        ];

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
            Mode::Frame(f) => {
                vec!["-f".to_owned(), f.to_string()]
            }
            // Render the whole animation using all the settings saved in the blend-file.
            Mode::Animation => {
                vec!["-a".to_owned()]
            }
            Mode::Section { start, end } => vec![
                "-s".to_owned(),
                start.to_string(),
                "-e".to_owned(),
                end.to_string(),
            ],
        };

        col.append(&mut additional_args);

        // Cycles add-on options must be specified following a double dash.
        if Some(Engine::Cycles) == self.engine {
            if let Some(device) = &self.device {
                col.push("-- --cycles-device".to_owned());
                col.push(device.to_string());
            }
        }

        col
    }
}
