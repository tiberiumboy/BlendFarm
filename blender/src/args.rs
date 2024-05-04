// use crate::device::Device;
// use crate::engine::Engine;
// use crate::format::Format;
use crate::mode::Mode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug, Serialize, Deserialize)]
pub struct Args {
    pub file: PathBuf, // required
    // pub engine: Option<Engine>, // optional
    // pub device: Option<Device>, // optional
    // pub format: Format,  // optional
    pub output: PathBuf, // optional
    pub mode: Mode,      // required
}

impl Args {
    pub fn new(file: PathBuf, output: PathBuf, mode: Mode) -> Self {
        Args {
            file,
            // engine: None,
            // device: None,
            output,
            mode,
        }
    }

    pub fn create_arg_list(&self) -> Vec<String> {
        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        /*
        -F :format::Format
        -x // use extension
        # is substitute to 0 pad, none will add to suffix four pounds (####)
        */
        let mut col = vec![
            "-b".to_owned(),
            self.file.to_str().unwrap().to_string(),
            "-o".to_owned(),
            self.output.to_str().unwrap().to_string(),
        ];

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

        col
    }
}
