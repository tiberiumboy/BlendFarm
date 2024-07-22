/*

Developer blog:
- Re-reading through this code several times, it seems like I got the bare surface working to get started with the rest of the components.
- Thought about if other computer node have identical os/arch - then just distribute the blender downloaded on the source to those target machine instead to prevent multiple downloads from the source.
- I eventually went back and read some parts of Rust Programming Language book to get a better understanding how to handle errors effectively.
- Using thiserror to define custom error within this library and anyhow for main.rs function, eventually I will have to handle those situation of the error message.

- Invoking blender should be called asyncronously on OS thread level. You have the ability to set priority for blender.
- Had to add BlenderJSON because some fields I could not deserialize/serialize - Which make sense that I don't want to share information that is only exclusive for the running machine to have access to.
    Instead BlenderJSON will only hold key information to initialize a new channel when accessed.

TODO:
private and public method are unorganized.
    - Consider reviewing them and see which method can be exposed publicly?
*/

use crate::{
    manager::{Manager, ManagerError},
    models::{
        args::Args, blender_peek_response::BlenderPeekResponse,
        blender_render_setting::BlenderRenderSetting, download_link::DownloadLink, status::Status,
    },
    page_cache::PageCacheError,
};
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env::temp_dir,
    fs,
    io::{self, BufRead, BufReader},
    path::{self, Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};
use thiserror::Error;

// TODO: consider making this private to make it easy to modify internally than affecting exposed APIs
#[derive(Debug, Error)]
pub enum BlenderError {
    #[error("Unsupported OS: {0}")]
    UnsupportedOS(String),
    #[error("Unsupported Architecture: {0}")]
    UnsupportedArch(String),
    #[error("Path to executable not found! {0}")]
    ExecutableNotFound(PathBuf),
    #[error("Unable to call blender!")]
    ExecutableInvalid,
    #[error(transparent)]
    PageCache(#[from] PageCacheError),
    #[error("Unable to render! Error: {0}")]
    RenderError(String),
    #[error("Unable to launch blender! Received Python errors: {0}")]
    PythonError(String),
    #[error("IO Error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
    #[error("Blender Manager error: {source}")]
    ManagerError {
        #[from]
        source: ManagerError,
    },
    #[error("Serde Error: {source}")]
    Serde {
        #[from]
        source: serde_json::Error,
    },
}

/// Blender structure to hold path to executable and version of blender installed.
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
    /// Create a new blender struct with provided path and version. Note this is not checked and enforced!
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

    /// fetch the blender executable path, used to pass into Command::process implementation
    pub fn get_executable(&self) -> &PathBuf {
        &self.executable
    }

    /// fetch the version of blender
    pub fn get_version(&self) -> &Version {
        &self.version
    }

    /// Return the extension expected to run on this operating system
    pub fn get_extension() -> Result<String, BlenderError> {
        match Manager::get_extension() {
            Ok(ext) => Ok(ext),
            Err(e) => Err(BlenderError::ManagerError { source: e }),
        }
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
        if !path.exists() {
            return Err(BlenderError::ExecutableNotFound(path.to_path_buf()));
        }

        // macOS is special. To invoke the blender application, I need to navigate inside Blender.app, which is an app bundle that contains stuff to run blender.
        // Command::Process needs to access the content inside app bundle to perform the operation correctly.
        // To do this - I need to append additional path args to correctly invoke the right application for this to work.
        // TODO: Verify this works for Linux/window OS?
        let path = match std::env::consts::OS {
            "macos" => &path.join("Contents/MacOS/Blender"),
            _ => path,
        };

        // Obtain the version by invoking version command to blender directly.
        // This verify two things, we actually fetch blender's current version rather than arbitruary guessing it.
        // this also validate that the executable is functional and operational. If we can launch blender and fetch version, then this part of the library should work as expected.
        // Otherwise, return an error stating that we are unable to verify this blender integrity, and warn user about this incident.
        match Self::check_version(path) {
            Ok(version) => Ok(Self::new(path.to_path_buf(), version)),
            Err(e) => Err(e),
        }
    }

    /// Create a blender struct from compressed content of the files
    pub fn from_content(path: impl AsRef<Path>, folder_name: &str) -> Result<Self, BlenderError> {
        let path = match DownloadLink::extract_content(&path, folder_name) {
            Ok(path) => path,
            Err(e) => return Err(BlenderError::ManagerError { source: e }),
        };
        Blender::from_executable(path)
    }

    pub fn latest_version_available() -> Result<Version, BlenderError> {
        // in this case I need to contact Manager class or BlenderDownloadLink somewhere and fetch the latest blender information
        // but for now let's just return default value of 4.1.0 until we return back to this at future later code.
        Ok(Version::new(4, 1, 0))
    }

    /// Download blender from the internet and install it to the provided path.
    ///
    /// # Potential errors
    ///
    /// * Unable to fetch download from the source - You may have lost connection to the internet, or this computer is unable to fetch download.blender.org website.
    ///  Please check and validate that you can access to the internet so that this program can download the correct version of blender on the system.
    ///
    /// * Unsupported OS - In some extreme case, this program cannot run on operating system or architecture outside of blender support. Curretnly supporting 64 bit architecture (Linux/Windows/Mac Intel) or Apple Silicon (arm64 base)
    ///  Currently there are no plan to support different operating system (Freebird, Solaris, Android) with matching architecture (arm, x86_64, powerpc)
    ///  It is possible to support these unsupported operating system / architecture by downloading the source code onto the target machine, and compile directly.
    ///  However, for this scope of this project, I have no plans or intention on supporting that far of detail to make this possible. (Especially when I need to verify all other crates are compatible with the target platform/os)
    ///
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::download(Version::new(4,1,0), PathBuf::from("path/to/installation")).unwrap();
    /// ```
    pub fn download(
        version: Version,
        install_path: impl AsRef<Path>,
    ) -> Result<Blender, BlenderError> {
        let mut manager = Manager::load();
        let executable = manager.download(&version, install_path).unwrap();
        Ok(Blender::new(executable, version))
    }

    pub fn peek(&self, blend_file: impl AsRef<Path>) -> Result<BlenderPeekResponse, BlenderError> {
        let peek_path = temp_dir().join("peek.py");
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
        if let Ok(output) = Command::new(&self.executable).args(args).output() {
            let stdout = String::from_utf8(output.stdout).unwrap();
            let parse = stdout.split("\n").collect::<Vec<&str>>();
            let json = parse[0].to_owned();
            return match serde_json::from_str(&json) {
                Ok(response) => Ok(response),
                Err(e) => Err(BlenderError::Serde { source: e }),
            };
        };
        Err(BlenderError::ExecutableInvalid)
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
            let blend_info = &self.peek(&args.file).unwrap();
            let setting = BlenderRenderSetting::parse_from(&args, blend_info);
            let arr = vec![setting];
            let data = serde_json::to_string(&arr).unwrap();
            let tmp_path = temp_dir().join("blender_render.json");
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
