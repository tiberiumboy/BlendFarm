#![cfg(not(doctest))]
/*
Developer blog:

Spending time on replacing xml-rpc-rs due to maintainers not willing to replace rouille plugin that supports this implementations.
I would instead incorporate the functionality of XML-RPC protocol myself instead of relying third party packages.
Reading the wikipedia - https://en.wikipedia.org/wiki/XML-RPC#Usage - xml-rpc is done via simple http server.

Currently, there is no error handling situation from blender side of things. If blender crash, we will resume the rest of the code in attempt to parse the data.
    This will eventually lead to a program crash because we couldn't parse the information we expect from stdout.
    TODO: How can I stream this data better?

- As of Blender 4.2 - they introduced BLENDER_EEVEE_NEXT as a replacement to BLENDER_EEVEE.
    Will need to make sure I pass in the correct enum for version 4.2 and above.

- Spoke to Sheepit - another "Intranet" distribution render service (Closed source)
    - In order to get Render preview window, there needs to be a GPU context to attach to.
        Otherwise, we'll have to wait for the render to complete the process before sending the image back to the user.
    - They mention to enforce compute methods, do not mix cpu and gpu. (Why?)

Advantage:
- can support M-series ARM processor.
- Original tool Doesn't composite video for you - We can make ffmpeg wrapper? - This will be a feature but not in this level of implementation.
- LogicReinc uses JSON to load batch file - difficult to adjust frame(s) after job sent.
    I'm creating an IPC between this program and python to ask next frame. To improve actions over blender.

Disadvantage:
- Currently rely on python script to do custom render within blender.
    No interops/additional cli commands other than interops through bpy (blender python) package
    Instead of using JSON to send configuration to python/blender, we're using IPC to control next frame/batch to render(s).
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

pub use crate::manager::{Manager, ManagerError};
pub use crate::models::args::Args;
use crate::models::config::BlenderConfiguration;
use crate::models::event::BlenderEvent;
use xml_rpc::server::Handler;

#[cfg(test)]
use blend::Instance;
use lazy_regex::regex_captures;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::num::ParseIntError;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
};
use thiserror::Error;
use tokio::spawn;
use xml_rpc::server::Server;
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

    fn handle_parse(names: &str) -> Result<u64, BlenderError> {
        names
            .parse()
            .map_err(|e: ParseIntError| BlenderError::InvalidFile(e.to_string()))
    }

    /// Obtain the version by invoking version command to blender directly.
    /// This function will invoke the -v command to retrieve blender version information.
    /// This validate two things,
    /// 1: Blender's internal version is reliable
    /// 2: Executable is functional and operational
    /// Otherwise, return an error that we were unable to verify this custom blender integrity.
    ///
    /// # Errors
    /// * InvalidData - executable path do not exist or is invalid. Please verify that the path provided exist and not compressed.
    ///  This error also serves where the executable is unable to provide the blender version.
    // TODO: Find a better way to fetch version from stdout (Research for best practice to parse data from stdout)
    fn check_version(executable_path: impl AsRef<Path>) -> Result<Self, BlenderError> {
        let exec_path = executable_path.as_ref();
        let output = Command::new(exec_path)
            .arg("-v")
            .output()
            .map_err(|_| BlenderError::ExecutableInvalid)?;
        let stdout = String::from_utf8(output.stdout).unwrap();
        match regex_captures!(
            r"\(Blender (?<major>[0-9]).(?<minor>[0-9]).(?<patch>[0-9])\)",
            &stdout
        ) {
            Some((_, major, minor, patch)) => {
                let maj = Self::handle_parse(major)?;
                let min = Self::handle_parse(minor)?;
                let pat = Self::handle_parse(patch)?;
                let version = Version::new(maj, min, pat);
                let blender = Self::new(exec_path.to_path_buf(), version);
                Ok(blender)
            }
            None => Err(BlenderError::ExecutableInvalid),
        }
    }

    // the difference between this function and getting executable are
    // a) MacOs is special. Executable reference a path inside app bundle.
    // b) This returns valid dir location to open to for user to look at from file POV
    // TODO: Remove all of this unwrap nightmare.
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
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::from_executable(Pathbuf::from("../examples/")).unwrap();
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

        let blender = Self::check_version(path)?;
        Ok(blender)
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
    pub async fn render(
        &self,
        args: Args,
        get_next_frame: Handler,
    ) -> Result<Receiver<BlenderEvent>, BlenderError> {
        let port = 8081;
        let socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);

        // I'm not even sure why we have two mpsc here for setup_listening_blender to use?
        let (signal, listener) = mpsc::channel::<BlenderEvent>();

        let settings = args.parse_from(&self.version).to_owned();
        self.setup_listening_server(settings, listener, &socket, get_next_frame)
            .await?;

        let (rx, tx) = mpsc::channel::<BlenderEvent>();
        let blender = self.clone();

        spawn(async move {
            if let Err(e) = &blender
                .setup_listening_blender(&args, rx, signal, &socket)
                .await
            {
                println!("{e:?}");
            }
        });

        // channel to invoke commands to blender while blender is running.
        Ok(tx)
    }

    async fn setup_listening_server(
        &self,
        settings: BlenderConfiguration,
        listener: Receiver<BlenderEvent>,
        socket: &SocketAddrV4,
        _get_next_frame: Box<dyn FnMut(Params) -> XmlResponse + Send + Sync>,
    ) -> Result<(), BlenderError> {
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
        // I think in order for me to make this working example, I need to create a struct that is memory bound to different threads, and read when available. This isolate mutation of variable and object that needs to be thread-safetly.
        // TODO: remove expect() once we have this working again.
        let mut server = Server::new(socket.port()).expect("Unable to open socket for xml_rpc!");

        server.register(
            "next_render_queue".to_owned(),
            Box::new(|_| {
                // where/how can I tell my render counts?
                Ok(Value::Int(1).into())
            }),
        );
        /*
        server.register("next_render_queue".to_owned(), move |params| match get_next_frame() {
            Some(frame) => Ok(frame),

            // this is our only way to stop python script.
            None => Err(Fault::new(1, "No more frames to render!")),
        });
        */

        // let me understand this better.
        // In this listening server, I'm setting up a xml-rpc server to listen to all of the blender python script.
        // When blender calls fetch_info, we provide back the global_settings we have from job information.
        server.register(
            "fetch_info".to_owned(),
            Box::new(move |_| {
                // How come we're using unwrap? seems dangerous and sketchy
                match serde_json::to_string(&*global_settings.clone()) {
                    Ok(setting) => Ok(Value::String(setting).into()),
                    Err(e) => Err(Value::fault(-1, e.to_string())),
                }
            }),
        );

        // spin up XML-RPC server
        spawn(async move {
            loop {
                // if the program shut down or if we've completed the render, then we should stop the server
                match listener.try_recv() {
                    Ok(BlenderEvent::Exit) => break,
                    Err(e) => {
                        // Received "Empty"?
                        println!("Something happen? {e:?}");
                        server.poll()
                        // break;
                    }
                    e => {
                        println!("Listener received unconditionally: {e:?}");
                        server.poll()
                    }
                }
            }
        });

        Ok(())
    }

    // setup xml-rpc listening server for blender's IPC
    async fn setup_listening_blender(
        &self,
        args: &Args,
        rx: Sender<BlenderEvent>,
        signal: Sender<BlenderEvent>,
        socket: &SocketAddrV4,
    ) -> Result<(), BlenderError> {
        let col = &args.file.setup_args(socket)?;
        // TODO: Find a way to remove unwrap()
        let stdout = Command::new(self.get_executable())
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
        // What's the difference between rx and signal?
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
            // [ERR] Error: Engine 'BLENDER_EEVEE_NEXT' not available for scene 'Scene' (an add-on may need to be installed or enabled)
            line if line.starts_with("EXCEPTION:") => {
                // Why would this crash?
                if let Err(e) = signal.send(BlenderEvent::Exit) {
                    println!("Fail to send error! {e:?}\n{line}");
                }
                if let Err(e) = rx.send(BlenderEvent::Error(line.to_owned())) {
                    println!("Fail to send error! {e:?}\n{line}");
                }
            }

            line if line.starts_with("COMPLETED") => {
                signal.send(BlenderEvent::Exit).unwrap();
                rx.send(BlenderEvent::Exit).unwrap();
            }

            // When launch blender for the first time, it prints out the version number and the hash information about the build)
            line if line.starts_with("Blender ") => {
                // if the line reads "Blender quit", we should send BlenderEvent::Exit signal
                if line.eq_ignore_ascii_case("blender quit") {
                    rx.send(BlenderEvent::Exit).unwrap();
                } else {
                    rx.send(BlenderEvent::Log(line)).unwrap();
                }
                

            }

            // Blender prints out reading blender files, here we'll just log the info anyway (We already have the information)
            line if line.starts_with("Read blend: ") => {
                rx.send(BlenderEvent::Log(line)).unwrap();
            }

            line if line.starts_with("regiondata free error") => {
                rx.send(BlenderEvent::Warning(line)).unwrap()
            }

            line if line.starts_with("Color management: ") => {
                rx.send(BlenderEvent::Log(line)).unwrap();
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
