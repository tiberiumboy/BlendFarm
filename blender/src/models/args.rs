use crate::models::{device::Device, engine::Engine, format::Format, mode::Mode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Args {
    file: PathBuf,              // required
    output: PathBuf,            // optional
    mode: Mode,                 // required
    pub engine: Option<Engine>, // optional
    pub device: Option<Device>, // optional
    pub format: Option<Format>, // optional - default to Png
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
        }
    }

    // could this just be used for the crate itself?
    pub fn create_arg_list(&self) -> Vec<String> {
        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        // # is substitute to 0 pad, none will add to suffix four pounds (####)

        // yet another instance here?
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
    }
}
