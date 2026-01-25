/*
Developer blog:

Spending time on replacing xml-rpc-rs due to maintainers not willing to replace rouille plugin that supports this implementations.
I would instead incorporate the functionality of XML-RPC protocol myself instead of relying third party packages.
Reading the wikipedia - https://en.wikipedia.org/wiki/XML-RPC#Usage - xml-rpc is done via simple http server.

Currently, there is no error handling situation from blender side of things. If blender crash, we will resume the rest of the code in attempt to parse the data.
    This will eventually lead to a program crash because we couldn't parse the information we expect from stdout.
    Todo peek into stderr and see if

- As of Blender 4.2 - they introduced BLENDER_EEVEE_NEXT as a replacement to BLENDER_EEVEE. Will need to make sure I pass in the correct enum for version 4.2 and above.

- Spoke to Sheepit - another "Intranet" distribution render service (Closed source)
    - In order to get Render preview window, there needs to be a GPU context to attach to. Otherwise, we'll have to wait for the render to complete the process before sending the image back to the user.
    - They mention to enforce compute methods, do not mix cpu and gpu. (Why?)

Trial:
- try loading .dll from blender? See if it's possible?

Advantage:
- can support M-series ARM processor.
- Original tool Doesn't composite video for you - We can make ffmpeg wrapper? - This will be a feature but not in this level of implementation.
- LogicReinc uses JSON to load batch file - difficult to adjust frame(s) after job sent. I'm creating an IPC between this program and python to ask next frame. To improve actions over blender.

Disadvantage:
- Currently rely on python script to do custom render within blender.
    No interops/additional cli commands other than interops through bpy (blender python) package
    Instead of using JSON to send configuration to python/blender, we're using IPC to control next frame to render.
    Currently using Command::Process to invoke commands to blender. Would like to see if there's public API or .dll to interface into.

Challenges:
    Blender support tileX/Y, but gluing the image together is a new challenge - a 64K 24bits image would consume about 3Gb, and size exponentially grow from there.
    Have a look into NIP2 to stitch large images together - https://github.com/libvips/nip2
        TODO: Find a way to glue image async by image to image, buffer to buffer, flush out each image before loading new image and hold nothing in memory, store it all on disk instead.

WARN:
    From LogicReinc FAQ's:
        Q: Render fails due to Gdip
        A: You're running Linux or Mac but did not install libgdiplus and libc6-dev,
            install these and you should be good.

        Q:Render fails on Linux
        A:You may not have the required blender system dependencies. Easiest way to cover them all is to just run `apt-get install blender` to fetch them all.
            (It does not have to be an up2date blender package, its just for dependencies)

TODO:
    Q: My Blendfile requires special addons to be active while rendering, can I add these?
    A: Blendfarm has its own versions of Blender in the BlenderData directory, and it runs
        these versions always in factory startup, thus without any added addons. This is done
        on purpose to make sure the environment is not altered. Most addons don't have to be
        active during rendering as they generate geometry etc. If you really need this, make
        an issue and I see what I can do. However do realise that this may make the workflow
        less smooth. (As you may need to set up these plugins for every Blender version instead
        of just letting BlendFarm do all the work.
    */
extern crate xml_rpc;
use crate::blend_file::{BlendFile, SceneInfo};
pub use crate::manager::{Manager, ManagerError};
pub use crate::models::args::Args;
use crate::models::blender_scene::BlenderScene;
use crate::models::config::BlenderConfiguration;
use crate::models::event::BlenderEvent;
use crate::models::peek_response::PeekResponse;
use crate::models::render_setting::RenderSetting;

#[cfg(test)]
use blend::Instance;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
};
use thiserror::Error;
use tokio::spawn;
use xml_rpc::Server;
use xml_rpc::{Params, Value, XmlResponse};

pub type Frame = i32;

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
    #[error("Unable to fetch info from blender home service! Are you connected to the internet and is blender foundation still around?")]
    ServiceOffline,
}

/// Blender structure to hold path to executable and version of blender installed.
/// Pretend this is the wrapper to interface with the actual blender program.
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf,
    /// Version of blender installed on the system.
    version: Version,
}

impl PartialEq for Blender {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }
}

impl PartialOrd for Blender {
    fn ge(&self, other: &Self) -> bool {
        self.version.ge(&other.version)
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.version.partial_cmp(&other.version)
    }
}

impl Ord for Blender {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.version.cmp(&other.version)
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

    // the difference between this function and getting executable are
    // a) MacOs is special. Executable reference a path inside app bundle.
    // b) This returns valid dir location to open to for user to look at from file POV
    pub fn get_relative_path(&self) -> &Path {
        if cfg!(target_os = "macos") {
            &self
                .executable
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
        } else {
            &self.executable.parent().unwrap()
        }
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
        // TODO: this is ugly, and I want to get rid of this. How can I improve this?
        // Backstory: Win and linux can be invoked via their direct app link. However, MacOS .app is just a bundle, which contains the executable inside.
        // To run process::Command, I must properly reference the executable path inside the blender.app on MacOS, using the hardcoded path below.
        const MACOS_PATH: &str = "Contents/MacOS/Blender";

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

        // this should be clear and explicit that I must have a valid path?
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

    // this is used to read and see blend file friendly view mode
    #[cfg(test)]
    #[allow(dead_code)]
    fn explore_value<'a>(obj: &Instance<'a>) {
        for i in &obj.fields {
            match i.1.is_primitive {
                true => {
                    match i.1.info {
                        blend::parsers::field::FieldInfo::Value => {
                            match i.1.type_name.as_str() {
                                "int" => {
                                    println!("{}: {} = {} ", i.0, i.1.type_name, &obj.get_i32(i.0));
                                }
                                "short" => {
                                    println!("{}: {} = {} ", i.0, i.1.type_name, &obj.get_u16(i.0));
                                }
                                "char" => {
                                    println!(
                                        "{}: {} = {} ",
                                        i.0,
                                        i.1.type_name,
                                        &obj.get_string(i.0)
                                    );
                                }
                                "float" => {
                                    println!("{}: {} = {}", i.0, i.1.type_name, &obj.get_f32(i.0));
                                }
                                "uint64_t" => {
                                    println!("{}: {} = {}", i.0, i.1.type_name, &obj.get_u64(i.0));
                                }
                                _ => println!("Unhandle value for {} | {}", i.1.type_name, i.0),
                            };
                        }
                        blend::parsers::field::FieldInfo::ValueArray { .. } => {
                            match i.1.type_name.as_str() {
                                "char" => {
                                    println!("{}: String = {}", i.0, &obj.get_string(i.0));
                                }
                                "float" => {
                                    println!("{}: vec<f32> = {:?}", i.0, &obj.get_f32_vec(i.0));
                                }
                                _ => {
                                    println!("Unhandle Value Array for {} | {}", i.1.type_name, i.0)
                                }
                            }
                        }
                        // blend::parsers::field::FieldInfo::PointerArray { .. } => todo!(),
                        // blend::parsers::field::FieldInfo::Pointer { indirection_count } => todo!(),
                        // blend::parsers::field::FieldInfo::FnPointer => todo!(),
                        _ => {
                            println!("Unhandle: {} | {} ", i.0, i.1.type_name)
                        }
                    }
                }
                false => {
                    println!("{}: TYPE = {}", i.0, i.1.type_name);
                }
            }
        }
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
    // issue here is that we need to lock thread. If we are rendering, we need to be able to call abort.
    pub async fn render<F>(&self, args: Args, get_next_frame: F) -> Receiver<BlenderEvent>
    where
        F: Fn() -> Option<i32> + Send + Sync + 'static,
    {
        let (signal, listener) = mpsc::channel::<BlenderEvent>();
        let blend_info: PeekResponse = args.file.peek_response(&self.version);
        // this is the only place used for BlenderRenderSetting... thoughts?
        let settings = BlenderConfiguration::parse_from(&args, &blend_info, &self.version);
        self.setup_listening_server(settings, listener, get_next_frame)
            .await;

        let (rx, tx) = mpsc::channel::<BlenderEvent>();
        let executable = self.executable.clone();

        spawn(async move {
            Blender::setup_listening_blender(&args, executable, rx, signal).await;
        });

        // channel to invoke commands to blender while blender is running.
        tx
    }

    fn next_render_queue_callback(params: Params) -> XmlResponse {
        // here, they're asking for next render queue callback.
        // in this case here, we don't care about the params, ? Why is Params called?

        XmlResponse::Ok(Params::new(vec![Value::Int(42)]))
    }

    async fn setup_listening_server<F>(
        &self,
        settings: BlenderConfiguration,
        listener: Receiver<BlenderEvent>,
        get_next_frame: F,
    ) -> Result<(), BlenderError>
    where
        F: Fn() -> Option<i32> + Send + Sync + 'static,
    {
        // Read here - https://en.wikipedia.org/wiki/XML-RPC#Usage
        /*
        In XML-RPC, a client performs an RPC by sending an HTTP request
        to a server that implements XML-RPC and receives the HTTP response.

        A call can have multiple parameters and one result.
        The protocol defines a few data types for the parameters and result.
        Some of these data types are complex, i.e. nested. For example,
            you can have a parameter that is an array of five integers.

        The parameters/result structure and the set of data types are meant to
        mirror those used in common programming languages.

        Identification of clients for authorization purposes can be achieved
        using popular HTTP security methods. Basic access authentication
        can be used for identification and authentication.

        In comparison to RESTful protocols, where resource representations (documents)
        are transferred, XML-RPC is designed to call methods. The practical difference
        is just that XML-RPC is much more structured, which means common library code
        can be used to implement clients and servers and there is less design and
        documentation work for a specific application protocol.

        [citation needed] One salient technical difference between typical RESTful
        protocols and XML-RPC is that many RESTful protocols use the HTTP URI
        for parameter information, whereas with XML-RPC, the URI just identifies the server.
        */

        let global_settings = Arc::new(settings);
        let socket = 8081;

        let mut server = Server::new(socket).expect("Unable to open socket for xml_rpc!");

        // while we're actively listening to the server, we can send response back.

        // subscribe mesages with invoker
        server.register(
            "next_render_queue".to_owned(),
            move |params| match get_next_frame() {
                Some(frame) => XmlResponse::Ok(Params::new(vec![Value::Int(frame)])),
                // this is our only way to stop python script.
                None => XmlResponse::Err(Fault::new(1, "No more frames to render!")),
            },
        );

        // server.register("fetch_info".to_owned(), move |_i: i32| {
        //     let setting = serde_json::to_string(&*global_settings.clone()).unwrap();
        //     Ok(setting)
        // });

        // spin up XML-RPC server
        spawn(async move {
            loop {
                // if the program shut down or if we've completed the render, then we should stop the server
                match listener.try_recv() {
                    Ok(BlenderEvent::Exit) => break,
                    e => println!("Listener received unconditionally: {e:?}"),
                    // _ => server.poll(),
                }
            }
        });

        Ok(())
    }

    fn setup_args(blend_file: &BlendFile) -> Result<Vec<String>, BlenderError> {
        let script_path = Blender::get_config_path().join("render.py");
        if !script_path.exists() {
            let data = include_bytes!("./render.py");
            fs::write(&script_path, data).map_err(|e| BlenderError::PythonError(e.to_string()))?;
        }

        let path = blend_file.to_path().as_os_str();

        Ok(vec![
            "--factory-startup".to_owned(),
            "-noaudio".into(),
            "-b".into(),
            path.to_str().unwrap().to_owned(),
            "-P".into(),
            script_path.to_str().unwrap().into(),
        ])
    }

    async fn setup_listening_blender<T: AsRef<Path>>(
        args: &Args,
        executable: T,
        rx: Sender<BlenderEvent>,
        signal: Sender<BlenderEvent>,
    ) -> Result<(), BlenderError> {
        let col = Self::setup_args(&args.file)?;

        // TODO: Find a way to remove unwrap()
        let stdout = Command::new(executable.as_ref())
            .args(col)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .stdout
            .unwrap();

        let reader = BufReader::new(stdout);

        // parse stdout for human to read
        let mut frame: i32 = 0;

        reader.lines().for_each(|line| {
            if let Ok(line) = line {
                Self::handle_blender_stdio(line, &mut frame, &rx, &signal);
            };
        });

        Ok(())
    }

    // TODO: This function updates a value above this scope -> See if we can just return the value instead?
    // TODO: Can we use stream instead? how can we parse data from blender into recognizable style?
    fn handle_blender_stdio(
        line: String,
        frame: &mut i32,
        rx: &Sender<BlenderEvent>,
        signal: &Sender<BlenderEvent>,
    ) {
        match line {
            // TODO: find a more elegant way to parse the string std out and handle invocation action.
            line if line.contains("Fra:") => {
                let col = line.split('|').collect::<Vec<&str>>();

                // this seems a bit expensive?
                let init = col[0].split(" ").next();
                if let Some(value) = init {
                    *frame = value.replace("Fra:", "").parse().unwrap_or(*frame);
                }
                let last = col.last().unwrap().trim();
                let slice = last.split(' ').collect::<Vec<&str>>();
                let msg = match slice[0] {
                    "Rendering" => {
                        let current = slice[1].parse::<f32>().unwrap();
                        let total = slice[3].parse::<f32>().unwrap();
                        BlenderEvent::Rendering { current, total }
                    }
                    _ => BlenderEvent::Unhandled(line),
                };
                rx.send(msg).unwrap();
            }

            line if line.starts_with("Time:") => {
                rx.send(BlenderEvent::Log(line)).unwrap();
            }
            // Python logs get injected to stdio
            line if line.starts_with("SUCCESS:") => {
                // somehow I received an error from sending?
                rx.send(BlenderEvent::Log(line)).unwrap();
            }
            line if line.starts_with("LOG:") => {
                rx.send(BlenderEvent::Log(line)).unwrap();
            }
            line if line.contains("Use:") => {
                rx.send(BlenderEvent::Log(line)).unwrap();
            }
            line if line.contains("RENDER_START:") => {
                rx.send(BlenderEvent::Log(line)).unwrap();
            }

            // it would be nice if we can somehow make this as a struct or enum of types?
            line if line.contains("Saved:") => {
                // TODO: Test this for OSX compatibility
                let location = line.split('\'').collect::<Vec<&str>>();
                let result = PathBuf::from(location[1]);
                rx.send(BlenderEvent::Completed {
                    frame: *frame,
                    result,
                })
                .unwrap();
            }

            // Strange how this was thrown, but doesn't report back to this program?
            line if line.starts_with("EXCEPTION:") => {
                signal.send(BlenderEvent::Exit).unwrap();
                rx.send(BlenderEvent::Error(line.to_owned())).unwrap();
            }

            line if line.starts_with("COMPLETED") => {
                signal.send(BlenderEvent::Exit).unwrap();
                rx.send(BlenderEvent::Exit).unwrap();
            }

            // TODO: Warning keyword is used multiple of times. Consider removing warning apart and submit remaining content above
            line if line.contains("Warning:") => {
                rx.send(BlenderEvent::Warning(line.to_owned())).unwrap();
            }

            line if line.contains("Error:") => {
                let msg = BlenderEvent::Error(line.to_owned());
                rx.send(msg).unwrap();
            }

            line if line.contains("Blender quit") => {
                // ignoring this...
                println!("Blender quit! Should we handle something about this here at this point of time?");
            }

            // any unhandle handler is submitted raw in console output here.
            line if !line.is_empty() => {
                // somehow it was able to pick up the blender version and commit hash value?
                let msg = format!("[Unhandle Blender Event]:{line}");
                let event = BlenderEvent::Unhandled(msg);
                rx.send(event).unwrap();
            }
            _ => {
                // Only empty log entry would show up here...
            }
        };
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
