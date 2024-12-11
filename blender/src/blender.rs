/*
Developer blog:
- Re-reading through this code several times, it seems like I got the bare surface working to get started with the rest of the components.
- Invoking blender should be called asyncronously on OS thread level. You have the ability to set priority for blender.

Decided to merge Manager codebase here as accessing from crate would make more sense, e.g. blender::Manager, instead of manager::Manager
- Although, I would like to know if it's possible to do mod alias so that I could continue to keep my manager class separate? Or should I just rely on mods?

Currently, there is no error handling situation from blender side of things. If blender crash, we will resume the rest of the code in attempt to parse the data.
    This will eventually lead to a program crash because we couldn't parse the information we expect from stdout.
    Todo peek into stderr and see if

- As of Blender 4.2 - they introduced BLENDER_EEVEE_NEXT as a replacement to BLENDER_EEVEE. Will need to make sure I pass in the correct enum for version 4.2 and above.

- Spoke to Sheepit - another "Intranet" distribution render service (Closed source)
    - In order to get Render preview window, there needs to be a GPU context to attach to. Otherwise, we'll have to wait for the render to complete the process before sending the image back to the user.
    - They mention to enforce compute methods, do not mix cpu and gpu. (Why?)

Trial:
- Try docker?
- try loading .dll from blender? See if it's possible?
- Learning Unsafe Rust and using FFI - going to try and find blender's library code that rust can bind to.
    - todo: see about cbindgen/cxx?

Advantage:
- can support M-series ARM processor.
- Original tool Doesn't composite video for you - We can make ffmpeg wrapper? - This will be a feature but not in this level of implementation.

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

    Q: My Blendfile requires special addons to be active while rendering, can I add these?
    A: Blendfarm has its own versions of Blender in the BlenderData directory, and it runs
        these versions always in factory startup, thus without any added addons. This is done
        on purpose to make sure the environment is not altered. Most addons don't have to be
        active during rendering as they generate geometry etc. If you really need this, make
        an issue and I see what I can do. However do realise that this may make the workflow
        less smooth. (As you may need to set up these plugins for every Blender version instead
        of just letting BlendFarm do all the work.
    */
pub use crate::manager::{Manager, ManagerError};
pub use crate::models::args::Args;
use crate::models::mode::Mode;
use crate::models::{
    blender_peek_response::BlenderPeekResponse, blender_render_setting::BlenderRenderSetting,
    status::Status,
};
use blend::Blend;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};
use thiserror::Error;
use tokio::spawn;

// TODO: this is ugly, and I want to get rid of this. How can I improve this?
// Backstory: Win and linux can be invoked via their direct app link. However, MacOS .app is just a bundle, which contains the executable inside.
// To run process::Command, I must properly reference the executable path inside the blender.app on MacOS, using the hardcoded path below.
const MACOS_PATH: &str = "Contents/MacOS/Blender";

#[derive(Debug, Error)]
pub enum BlenderError {
    #[error("Unable to call blender!")]
    ExecutableInvalid,
    #[error("Path to executable not found! {0}")]
    ExecutableNotFound(PathBuf),
    #[error("Invalid file path! {0}")]
    InvalidFile(String),
    #[error("Unable to render! Error: {0}")]
    RenderError(String),
    #[error("Unable to launch blender! Received Python errors: {0}")]
    PythonError(String),
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
    pub fn get_config_path() -> PathBuf {
        dirs::config_dir().unwrap().join("BlendFarm")
    }

    /// Return the executable path to blender (Entry point for CLI)
    pub fn get_executable(&self) -> &Path {
        &self.executable
    }

    /// Return validated Blender Version
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
    // TODO: Consider using blend library to read the data instead.
    pub async fn peek(blend_file: &PathBuf) -> Result<BlenderPeekResponse, BlenderError> {
        /*
        Experimental code, trying to use blend plugin to extract information rather than opening up blender for this.

        Problem: I can't seem to find a way to obtain the following information:
            - True scene name (Not SCScene)
            - True camera name (Not CACamera)
            - frame_start/end variable.
            - render_height/width variable.
        - denoiser/sample rate (From cycle?)
            - fps?
            // from peek.py
            RenderWidth = scn.render.resolution_x,
            RenderHeight = scn.render.resolution_y,
            FrameStart = scn.frame_start,
            FrameEnd = scn.frame_end,
            FPS = scn.render.fps,
            Denoiser = scn.cycles.denoiser,
            Samples = scn.cycles.samples,
            // do note here - we're capturing the OB name not the CA name!
            Cameras = scn.objects.obj.type["CAMERA"].obj.name,
            SelectedCamera = scn.camera.name,
            Scenes = bpy.data.scenes.scene.name
            SelectedScene = scn.name
            */

        let blend = Blend::from_path(&blend_file)
            .map_err(|_| BlenderError::InvalidFile("Received BlenderParseError".to_owned()))?;

        // blender version are display as three digits number, e.g. 404 is major: 4, minor: 4.
        // treat this as a u16 major = u16 / 100, minor = u16 % 100;
        let value: u64 = std::str::from_utf8(&blend.blend.header.version)
            .expect("Fail to parse version into utf8")
            .parse()
            .expect("Fail to parse string to value");
        let major = value / 100;
        let minor = value % 100;

        // using scope to drop manager usage.
        let blend_version = {
            let manager = Manager::load();

            // Get the latest patch from blender home
            match manager
                .home
                .as_ref()
                .iter()
                .find(|v| v.major.eq(&major) && v.minor.eq(&minor))
            {
                // TODO: Find a better way to handle this without using unwrap
                Some(v) => v.fetch_latest().unwrap().as_ref().clone(),
                // potentially could be a problem, if there's no internet connection, then we can't rely on zero patch?
                // For now this will do.
                None => Version::new(major.into(), minor.into(), 0),
            }
        };

        let mut scenes: Vec<String> = Vec::new();
        let mut cameras: Vec<String> = Vec::new();
        let mut frame_start: i32 = 0;
        let mut frame_end: i32 = 0;
        let mut render_width: i32 = 0;
        let mut render_height: i32 = 0;

        // this denotes how many scene objects there are.
        for obj in blend.instances_with_code(*b"SC") {
            let scene = obj.get("id").get_string("name").replace("SC", ""); // not the correct name usage?
            let render = &obj.get("r");

            // Grab the engine this scene is set to! This is useful!
            // let engine = &obj.get("r").get_string("engine"); // will show BLENDER_EEVEE_NEXT
            // let device = &render.get_i32("compositor_device"); // not sure how I can translate this to represent CPU/GPU? but currently show 0 for cpu

            // dbg!(device);
            // bpy.data.scenes["Scene2"].frame_start
            // render/output/properties/frame_range
            // dbg!(&render.get_i32("stamp"));
            render_width = render.get_i32("xsch");
            render_height = render.get_i32("ysch");
            frame_start = render.get_i32("sfra");
            frame_end = render.get_i32("efra");

            scenes.push(scene);
        }

        // interesting - I'm picking up the wrong camera here?
        for obj in blend.instances_with_code(*b"CA") {
            let camera = obj.get("id").get_string("name").replace("CA", "");
            cameras.push(camera);
        }

        let selected_camera = cameras.get(0).unwrap_or(&"".to_owned()).to_owned();
        let selected_scene = scenes.get(0).unwrap_or(&"".to_owned()).to_owned();

        let result = BlenderPeekResponse {
            last_version: blend_version,
            render_width,
            render_height,
            frame_start,
            frame_end,
            fps: 0,
            denoiser: "".to_owned(),
            samples: 0,
            cameras,
            selected_camera,
            scenes,
            selected_scene,
        };

        Ok(result)
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
    pub async fn render(&self, args: Args) -> Receiver<Status> {
        let (rx, tx) = mpsc::channel::<Status>();
        let executable = self.executable.clone();

        spawn(async move {
            // So far this part of the code works - but I'm getting an unusual error
            // I'm rececing an exception on stdout. [Errno 32] broken pipe?
            // thread panic here - err - Serde { source: Error("expected value", line: 1, column: 1) } ??
            // TODO: peek will be deprecated - See if we need to do anything different here?
            let blend_info = Self::peek(&args.file)
                .await
                .expect("Fail to parse blend file!"); // TODO: Need to clean this error up a bit.

            let tmp_path = Self::get_config_path().join("blender_render.json");
            let col = &args.create_arg_list(&tmp_path);
            let mut arr = Vec::new();
            match args.mode {
                // if we just need to render one frame.
                Mode::Frame(frame) => {
                    let entry = BlenderRenderSetting::parse_from(&args, frame, &blend_info);
                    arr.push(entry);
                }
                Mode::Animation { start, end } => {
                    for frame in start..=end {
                        let entry = BlenderRenderSetting::parse_from(&args, frame, &blend_info);
                        arr.push(entry);
                    }
                }
            };

            let data = serde_json::to_string(&arr).unwrap();
            fs::write(&tmp_path, data).unwrap();

            let stdout = Command::new(executable)
                .args(col)
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
                .stdout
                .unwrap();

            let reader = BufReader::new(stdout);
            let mut frame: i32 = 0;

            // parse stdout for human to read
            reader.lines().for_each(|line| {
                if let Ok(line) = line {
                    match line {
                        line if line.contains("Fra:") => {
                            let col = line.split('|').collect::<Vec<&str>>();

                            // this seems a bit expensive?
                            let init = col[0].split(" ").next();
                            if let Some(value) = init {
                                frame = value.replace("Fra:", "").parse().unwrap_or(1);
                            }
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
                        line if line.contains("Saved:") => {
                            let location = line.split('\'').collect::<Vec<&str>>();
                            let result = PathBuf::from(location[1]);
                            rx.send(Status::Completed { frame, result }).unwrap();
                        }
                        line if line.contains("Warning:") => {
                            rx.send(Status::Warning {
                                message: line.to_owned(),
                            })
                            .unwrap();
                        }
                        line if line.contains("Error:") => {
                            let msg = Status::Error(BlenderError::RenderError(line.to_owned()));
                            rx.send(msg).unwrap();
                        }
                        line if line.contains("Blender quit") => {
                            rx.send(Status::Exit).unwrap();
                        }
                        line if !line.is_empty() => {
                            let msg = Status::Running {
                                status: line.to_owned(),
                            };
                            rx.send(msg).unwrap();
                        }
                        _ => {
                            // Only empty log entry would show up here...
                        }
                    };
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
