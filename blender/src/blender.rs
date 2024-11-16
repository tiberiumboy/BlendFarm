/*

Developer blog:
- Re-reading through this code several times, it seems like I got the bare surface working to get started with the rest of the components.
- Thought about if other computer node have identical os/arch - then just distribute the blender downloaded on the source to those target machine instead to prevent multiple downloads from the source.
- I eventually went back and read some parts of Rust Programming Language book to get a better understanding how to handle errors effectively.
- Using thiserror to define custom error within this library and anyhow for main.rs function, eventually I will have to handle those situation of the error message.

- Invoking blender should be called asyncronously on OS thread level. You have the ability to set priority for blender.
- Had to add BlenderJSON because some fields I could not deserialize/serialize - Which make sense that I don't want to share information that is only exclusive for the running machine to have access to.
    Instead BlenderJSON will only hold key information to initialize a new channel when accessed.

Decided to merge Manager codebase here as accessing from crate would make more sense, e.g. blender::Manager, instead of manager::Manager
- Although, I would like to know if it's possible to do mod alias so that I could continue to keep my manager class separate? Or should I just rely on mods?

Currently, there is no error handling situation from blender side of things. If blender crash, we will resume the rest of the code in attempt to parse the data.
    This will eventually lead to a program crash because we couldn't parse the information we expect from stdout.
    Todo peek into stderr and see if


Trial:
- Try docker?
- try loading .dll from blender? See if it's possible?
- Learning Unsafe Rust and using FFI - going to try and find blender's library code that rust can bind to.
    - todo: see about cbindgen/cxx?

Advantage:
- can support M-series ARM processor.
- Original tool Doesn't composite video for you - We can make ffmpeg wrapper?

Disadvantage:
- Currently rely on python script to do custom render within blender.
    No interops/additional cli commands other than interops through bpy (blender python) package
    Currently using Command::Process to invoke commands to blender. Would like to see if there's public API or .dll to interface into.
        - Currently learning Low Level Programming to understand assembly and C interfaces.


WARN:
    From LogicReinc FAQ's:
        Q: Render fails due to Gdip
        A: You're running Linux or Mac but did not install libgdiplus and libc6-dev,
            install these and you should be good.

        Q:Render fails on Linux
        A:You may not have the required blender system dependencies. Easiest way to cover them all is to just run `apt-get install blender` to fetch them all.
            (It does not have to be an up2date blender package, its just for dependencies)

TODO:
private and public method are unorganized.
    - Consider reviewing them and see which method can be exposed publicly?
    - Find a way to make crate manager::Manager accessible via blender::Manager instead? This would make the code more clean and structured.

    Q: My Blendfile requires special addons to be active while rendering, can I add these?
    A: Blendfarm has its own versions of Blender in the BlenderData directory, and it runs
        these versions always in factory startup, thus without any added addons. This is done
        on purpose to make sure the environment is not altered. Most addons don't have to be
        active during rendering as they generate geometry etc. If you really need this, make
        an issue and I see what I can do. However do realise that this may make the workflow
        less smooth. (As you may need to set up these plugins for every Blender version instead
        of just letting BlendFarm do all the work.

    */
// #[cfg(feature = "manager")]
pub use crate::manager::{Manager, ManagerError};
pub use crate::models::args::Args;

use crate::models::{
    blender_peek_response::BlenderPeekResponse, blender_render_setting::BlenderRenderSetting,
    status::Status,
};
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{self, Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};
use thiserror::Error;
// TODO: this is ugly, and I want to get rid of this. How can I improve this?
// Backstory: Win and linux can be invoked via their direct app link. However, MacOS .app is just a bundle, which contains the executable inside.
// To run process::Command, I must properly reference the executable path inside the blender.app on MacOS, using the hardcoded path below.
const MACOS_PATH: &str = "Contents/MacOS/Blender";

#[derive(Debug, Error)]
pub enum BlenderError {
    #[error("Path to executable not found! {0}")]
    ExecutableNotFound(PathBuf),
    #[error("Unable to call blender!")]
    ExecutableInvalid,
    #[error("Unable to render! Error: {0}")]
    RenderError(String),
    #[error("Unable to launch blender! Received Python errors: {0}")]
    PythonError(String),
    // #[error("Ran into IO issue extracting contents")]
    // UnableToExtract,
}

/// Blender structure to hold path to executable and version of blender installed.
/// Pretend this is the wrapper to interface with the actual blender program.
#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Eq, Ord)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf, // Must validate before usage!
    /// Version of blender installed on the system.
    version: Version, // Private immutable variable - Must validate before using!
}

impl PartialEq for Blender {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }
}

impl Blender {
    /* Private method impl */

    /// Create a new blender struct with provided path and version. This does not checked and enforced!
    ///
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::new(PathBuf::from("path/to/blender"), Version::new(4,1,0));
    /// ```
    fn new(executable: PathBuf, version: Version) -> Self {
        Self {
            executable,
            version,
        }
    }

    /// This function will invoke the -v command ot retrieve blender version information.
    ///
    /// # Errors
    /// * InvalidData - executable path do not exist or is invalid. Please verify that the path provided exist and not compressed.
    ///  This error also serves where the executable is unable to provide the blender version.
    // TODO: Find a better way to fetch version from stdout (Research for best practice to parse data from stdout)
    fn check_version(executable_path: impl AsRef<Path>) -> Result<Version, BlenderError> {
        if let Ok(output) = Command::new(executable_path.as_ref()).arg("-v").output() {
            // wonder if there's a better way to test this?
            let regex =
                Regex::new(r"(Blender (?<major>[0-9]).(?<minor>[0-9]).(?<patch>[0-9]))").unwrap();

            let stdout = String::from_utf8(output.stdout).unwrap();
            return match regex.captures(&stdout) {
                Some(info) => Ok(Version::new(
                    info["major"].parse().unwrap(),
                    info["minor"].parse().unwrap(),
                    info["patch"].parse().unwrap(),
                )),
                None => Err(BlenderError::ExecutableInvalid),
            };
        }
        Err(BlenderError::ExecutableInvalid)
    }

    /// Fetch the configuration path for blender. This is used to store temporary files and configuration files for blender.
    fn get_config_path() -> PathBuf {
        dirs::config_dir().unwrap().join("Blender")
    }

    /* Public method impl */

    /// fetch the blender executable path, used to pass into Command::process implementation
    pub fn get_executable(&self) -> &PathBuf {
        &self.executable
    }

    /// fetch the version of blender
    pub fn get_version(&self) -> &Version {
        &self.version
    }

    /// Create a new blender struct from executable path. This function will fetch the version of blender by invoking -v command.
    /// Otherwise, if Blender is not install, or a version is not found, an error will be thrown
    ///
    /// # Error
    ///
    /// * InvalidData - executable path do not exist, or is invalid. Please verify that the executable path is correct and leads to the actual executable.
    /// *
    /// # Examples
    ///
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::from_executable(Pathbuf::from("path/to/blender")).unwrap();
    /// ```
    pub fn from_executable(executable: impl AsRef<Path>) -> Result<Self, BlenderError> {
        // check and verify that the executable exist.
        // first line for validating blender executable.
        let path = executable.as_ref();

        // macOS is special. To invoke the blender application, I need to navigate inside Blender.app, which is an app bundle that contains stuff to run blender.
        // Command::Process needs to access the content inside app bundle to perform the operation correctly.
        // To do this - I need to append additional path args to correctly invoke the right application for this to work.
        // TODO: Verify this works for Linux/window OS?
        let path = if std::env::consts::OS == "macos" && !&path.ends_with(MACOS_PATH) {
            &path.join(MACOS_PATH)
        } else {
            path
        };

        // this should be clear and explicit that I must have a valid path? How can I do this?
        // does it need a wrapper?
        if !path.exists() {
            return Err(BlenderError::ExecutableNotFound(path.to_path_buf()));
        }

        // Obtain the version by invoking version command to blender directly.
        // This validate two things,
        // 1: Blender's internal version is reliable
        // 2: Executable is functional and operational
        // Otherwise, return an error that we were unable to verify this custom blender integrity.
        let version = Self::check_version(path)?;
        Ok(Self::new(path.to_path_buf(), version))
    }

    /// Peek is a function design to read and fetch information about the blender file.
    /// To do this, we must have a valid blender executable path, and run the peek.py code to fetch a json response.
    pub fn peek(&self, blend_file: impl AsRef<Path>) -> Result<BlenderPeekResponse, BlenderError> {
        let peek_path = Self::get_config_path().join("peek.py");

        // if peek file does not exist - create one.
        if !peek_path.exists() {
            let bytes = include_bytes!("peek.py");
            fs::write(&peek_path, bytes).unwrap();
        }

        let full_path = path::absolute(blend_file).unwrap();
        let args = vec![
            "--factory-startup".to_owned(),
            "-noaudio".to_owned(),
            "-b".to_owned(),
            full_path.to_str().unwrap().to_owned(),
            "-P".to_owned(),
            peek_path.to_str().unwrap().to_owned(),
        ];
        let output = Command::new(&self.executable)
            .args(args)
            .output()
            .map_err(|_| BlenderError::ExecutableInvalid)?;

        let stdout = String::from_utf8(output.stdout).unwrap();
        let parse = stdout.split("\n").collect::<Vec<&str>>();
        let json = parse[0].to_owned();

        serde_json::from_str(&json).map_err(|e| BlenderError::PythonError(e.to_string()))
    }

    /// Render one frame - can we make the assumption that ProjectFile may have configuration predefined Or is that just a system global setting to apply on?
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// use blender::args::Args;
    /// let blender = Blender::from_executable("path/to/blender").unwrap();
    /// let args = Args::new(PathBuf::from("path/to/project.blend"), PathBuf::from("path/to/output.png"));
    /// let final_output = blender.render(&args).unwrap();
    /// ```
    // so instead of just returning the string of render result or blender error, we'll simply use the single producer to produce result from this class.
    pub fn render(self, args: Args) -> Receiver<Status> {
        let (rx, tx) = mpsc::channel::<Status>();
        thread::spawn(move || {
            // So far this part of the code works - but I'm getting an unusual error
            // I'm rececing an exception on stdout. [Errno 32] broken pipe?
            // thread panic here - err - Serde { source: Error("expected value", line: 1, column: 1) } ??
            // why are we having issue trying to peek into this?
            let blend_info = &self.peek(&args.file).unwrap();
            let setting = BlenderRenderSetting::parse_from(&args, blend_info);
            let arr = vec![setting];
            let data = serde_json::to_string(&arr).unwrap();
            let tmp_path = Self::get_config_path().join("blender_render.json");
            fs::write(&tmp_path, data).unwrap();
            let col = &args.create_arg_list(tmp_path);

            let stdout = Command::new(&self.executable)
                .args(col)
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
                .stdout
                .unwrap();

            let reader = BufReader::new(stdout);

            // parse stdout for human to read
            reader.lines().for_each(|line| {
                let line = line.unwrap();

                if line.is_empty() {
                    return;
                }

                // I feel like there's a better way of handling this? Yes!
                match &line {
                    line if line.contains("Fra:") => {
                        let col = line.split('|').collect::<Vec<&str>>();
                        let last = col.last().unwrap().trim();
                        let slice = last.split(' ').collect::<Vec<&str>>();
                        let msg = match slice[0] {
                            "Rendering" => {
                                let current = slice[1].parse::<f32>().unwrap();
                                let total = slice[3].parse::<f32>().unwrap();
                                let percentage = current / total * 100.0;
                                let render_perc = format!("{} {:.2}%", last, percentage);
                                Status::Running {
                                    status: render_perc,
                                }
                            }
                            "Sample" => Status::Running {
                                status: last.to_owned(),
                            },
                            _ => Status::Log {
                                status: last.to_owned(),
                            },
                        };
                        rx.send(msg).unwrap();
                    }
                    // If blender completes the saving process then we should return the path
                    line if line.contains("Saved:") => {
                        let location = line.split('\'').collect::<Vec<&str>>();
                        let path = PathBuf::from(location[1]);
                        let msg = Status::Completed { result: path };
                        rx.send(msg).unwrap();
                    }
                    line if line.contains("Warning:") => {
                        let msg = Status::Warning {
                            message: line.to_owned(),
                        };
                        rx.send(msg).unwrap();
                    }
                    line if line.contains("Error:") => {
                        let msg = Status::Error(BlenderError::RenderError(line.to_owned()));
                        rx.send(msg).unwrap();
                    }
                    // ("Warning:"..) => println!("{}", line),
                    line if !line.is_empty() => {
                        // do not send info if line is empty!
                        let msg = Status::Running {
                            status: line.to_owned(),
                        };
                        rx.send(msg).unwrap();
                    }
                    _ => {
                        // Only empty log entry would show up here...
                    }
                };
            });
        });
        tx
    }
}

// TODO: impl unit test for blender specifically.
/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_run() {}

    #[test]
    fn should_render() {}
}
*/
