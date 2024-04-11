use crate::models::{project_file::ProjectFile, server_setting::ServerSetting};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env, io,
    path::PathBuf,
    process::{Command, Output},
};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Engine {
    Cycles,
    Eevee,
    Workbench,
}

// TODO: Once I figure out about getting blender configuration from the hardware, use this to return back to the host about this machine configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Device {
    CPU,
    CUDA,
    OPTIX,
    HIP,
    ONEAPI,
    METAL,
}
// append +CPU to gpu to include CPU into render cycle.

// const CACHE_DAYS: u8 = 3;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blender {
    url: Option<Url>, // URL to download the program (cache)
    dl_content: Option<PathBuf>,
    executable: Option<PathBuf>, // path to the blender executable
    version: Version,            // version of blender
    engine: Engine,
    device: Device,
}

// url to download repository
const VERSIONS_URL: &str = "https://download.blender.org/release/";

fn get_os() -> String {
    match env::consts::OS {
        "linux" => "linux64".to_owned(),
        "windows" => "windows64".to_owned(),
        "macos" => "macOS".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn get_ext() -> String {
    match env::consts::OS {
        "linux" => "tar.xz".to_owned(),
        "windows" => "zip".to_owned(),
        "macos" => "dmg".to_owned(),
        _ => "unknown".to_owned(),
    }
}

impl Blender {
    pub fn from_executable(executable: PathBuf) -> std::io::Result<Self> {
        // this should return the version number
        // macos
        let exec = executable.to_str().unwrap();
        let output = match Self::exec_command(&exec, "-v") {
            Ok(output) => {
                let stdout = String::from_utf8(output.stdout.clone()).unwrap();
                let parts = stdout.split("\n\t");
                let collection = &parts.collect::<Vec<&str>>();
                let first = collection.first().unwrap();
                dbg!(&collection);
                // if first.contains("Blender") {
                //     Version::parse(&first[8..]).unwrap()
                // } else {
                Version::new(2, 93, 0)
                // }
                // still sketchy, but it'll do for now
            }
            _ => Version::new(2, 93, 0), // TODO: Find a better way to handle this
        };

        Ok(Self {
            url: None,
            version: output,
            dl_content: None,
            executable: Some(executable),
            engine: Engine::Cycles,
            device: Device::CPU,
        })
    }

    fn exec_command(executable: &str, args: &str) -> Result<Output, std::io::Error> {
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg(executable)
                .arg(args)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(executable)
                .arg(args)
                .output()
        }
    }

    pub fn from_url(url: Url) -> Self {
        let version = Version::new(2, 93, 0);
        let dl_content = None;
        let executable = None;

        Self {
            url: Some(url),
            version,
            dl_content,
            executable,
            engine: Engine::Cycles,
            device: Device::CPU,
        }
    }

    pub fn from_cache(dl_content: PathBuf) -> Self {
        let version = Version::new(2, 93, 0);
        let url = Url::parse(VERSIONS_URL).unwrap();
        let executable = None;

        Self {
            url: Some(url),
            version,
            dl_content: Some(dl_content),
            executable,
            engine: Engine::Cycles,
            device: Device::CPU,
        }
    }

    pub fn from_version(version: Version) -> Self {
        let url = Url::parse(VERSIONS_URL).unwrap();
        let dl_content = None;
        let executable = None;

        Self {
            url: Some(url),
            version,
            dl_content,
            executable,
            device: Device::CPU,
            engine: Engine::Cycles,
        }
    }

    fn is_installed(&self) -> bool {
        self.executable.is_some()
    }

    fn is_cached(&self) -> bool {
        self.dl_content.is_some()
    }

    fn download(&mut self) -> Result<(), io::Error> {
        if self.is_cached() {
            return Ok(());
        }

        let config = ServerSetting::default();
        let archive_name = format!("{}-{}.{}", self.version, get_os(), get_ext());
        let archive_path = PathBuf::from(&config.blender_data.path).join(&archive_name);

        dbg!(&archive_path);

        self.dl_content = Some(archive_path);

        // download the file

        Ok(())
    }

    fn parse(base_url: &Url, version: &Version) -> Self {
        let dir = format!("Blender{}/", version);
        let result = base_url.join(&dir);
        dbg!(&result);

        Self {
            url: result.ok(),
            version: version.clone(),
            dl_content: None,
            executable: None,
            engine: Engine::Cycles,
            device: Device::CPU,
        }
    }

    fn get_file_name(&self) -> String {
        let os = env::consts::OS;
        let ext = "tar.xy";
        // todo - correct arch labeling, e.g. x86_64 -> x64, arm -> arm64, etc
        let arch = env::consts::ARCH;
        let archive = format!("blender-{}-{os}-{arch}.{ext}", self.version);

        dbg!(archive);

        "Testing something here".to_owned()
        // format!(args)
    }

    fn get_executable(&self) -> &str {
        // we'll find a way to get the executable if all else fails
        self.executable.as_ref().unwrap().to_str().unwrap()
    }

    pub fn render(&mut self, project: &ProjectFile, frame: i32) -> io::Result<PathBuf> {
        if !self.is_installed() {
            let _ = &self.download().expect("Fail to download blender!");
        }

        let path = project
            .file_path() // Project
            .to_str()
            .unwrap();

        let config = ServerSetting::default();
        let mut tmp = config.blender_data.path.clone();
        tmp.push(format!("{}_{}.png", project.file_name, frame));
        dbg!(&tmp);

        // let output = ServerSetting::default(); //project.output.as_os_str().to_str().unwrap();
        let output = tmp.as_os_str().to_str().unwrap();

        /*
        "--factory-startup", // skip startup.blend
        "-noaudio",          // no sound
        "-b",                // background
        path,
        "-o", // output
        output,
        // --log "*" to log everything
        "-f", // frame (must be last!)
        */
        let cmd = format!(
            "--factory-startup -noaudio -b {} -o {} -f {}",
            path, output, frame
        );

        dbg!(&cmd);

        // we'll figure out what to do with this output...
        let exec_path = self.executable.as_ref().unwrap().clone();
        let path_as_str = exec_path.to_str().unwrap();
        let _output = Self::exec_command(path_as_str, &cmd).unwrap();

        // display the output and see what result we'll get
        dbg!(_output);

        Ok(tmp)
    }
}
