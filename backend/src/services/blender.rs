use crate::models::{project_file::ProjectFile, server_setting::ServerSetting};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env,
    io::Result,
    marker::PhantomData,
    path::PathBuf,
    process::{Command, Output},
};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct NotInstalled;

#[derive(Debug, Deserialize, Serialize)]
pub struct Installed;

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub enum Engine {
//     Cycles,
//     Eevee,
//     Workbench,
// }

// TODO: Once I figure out about getting blender configuration from the hardware, use this to return back to the host about this machine configuration
// #[derive(Clone, Debug, Serialize, Deserialize)]
// enum Device {
//     CPU,
//     CUDA,
//     OPTIX,
//     HIP,
//     ONEAPI,
//     METAL,
// }
// append +CPU to gpu to include CPU into render cycle.

// const CACHE_DAYS: u8 = 3;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blender<State = NotInstalled> {
    url: Option<Url>, // URL to download the program (cache)
    dl_content: Option<PathBuf>,
    version: Version, // version of blender
    executable: Option<PathBuf>,
    state: PhantomData<State>,
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

pub fn get_ext() -> String {
    match env::consts::OS {
        "linux" => "tar.xz".to_owned(),
        "windows" => "zip".to_owned(),
        "macos" => "dmg".to_owned(),
        _ => "unknown".to_owned(),
    }
}

impl Blender {
    pub fn from_executable(executable: PathBuf) -> Result<Blender<Installed>> {
        // this should return the version number
        // macos
        let exec = executable.to_str().unwrap();
        let output = Command::new(exec)
            .arg("-v")
            .output()
            .expect("Failed to execute command!");

        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        let collection = stdout.split("\n\t").collect::<Vec<&str>>();
        let first = collection.first().unwrap();
        let version = if first.contains("Blender") {
            Version::parse(&first[8..]).unwrap()
        } else {
            Version::new(4, 1, 0)
        };
        // still sketchy, but it'll do for now

        Ok(Blender {
            url: None,
            dl_content: None,
            executable: Some(executable),
            version,
            state: PhantomData::<Installed>,
        })
    }

    #[allow(dead_code)]
    pub fn from_url(url: Url) -> Blender<NotInstalled> {
        let version = Version::new(2, 93, 0);
        let dl_content = None;

        Blender {
            url: Some(url),
            version,
            dl_content,
            executable: None,
            state: PhantomData::<NotInstalled>,
        }
    }

    #[allow(dead_code)]
    pub fn from_cache(dl_content: PathBuf) -> Blender<NotInstalled> {
        let version = Version::new(2, 93, 0);
        let url = Url::parse(VERSIONS_URL).unwrap();

        Blender {
            url: Some(url),
            version,
            dl_content: Some(dl_content),
            executable: None,
            state: PhantomData::<NotInstalled>,
        }
    }

    pub fn from_version(version: Version) -> Blender<NotInstalled> {
        let url = Url::parse(VERSIONS_URL).unwrap();
        let dl_content = None;

        Blender {
            url: Some(url),
            version,
            dl_content,
            executable: None,
            state: PhantomData::<NotInstalled>,
        }
    }

    #[allow(dead_code)]
    fn parse(base_url: &Url, version: &Version) -> Blender<NotInstalled> {
        let dir = format!("Blender{}/", version);
        let result = base_url.join(&dir);
        dbg!(&result);

        Blender {
            url: result.ok(),
            version: version.clone(),
            dl_content: None,
            executable: None,
            state: PhantomData::<NotInstalled>,
        }
    }
}

impl<State> Blender<State> {
    fn is_installed(&self) -> bool {
        self.executable.is_some()
    }

    fn is_cached(&self) -> bool {
        self.dl_content.is_some()
    }

    fn get_file_name(&self) -> String {
        let os = env::consts::OS;
        let ext = match env::consts::OS {
            "linux" => "tar.xz".to_owned(),
            "windows" => "zip".to_owned(),
            "macos" => "dmg".to_owned(),
            _ => "unknown".to_owned(),
        };
        // todo - correct arch labeling, e.g. x86_64 -> x64, arm -> arm64, etc
        let arch = env::consts::ARCH;
        let archive = format!("blender-{}-{os}-{arch}.{ext}", self.version);

        dbg!(archive);

        "Testing something here".to_owned()
        // format!(args)
    }
}

impl Blender<Installed> {
    fn get_executable(&self) -> &str {
        // we'll find a way to get the executable if all else fails
        self.executable.as_ref().unwrap().to_str().unwrap()
    }

    fn exec_command(&self, args: &str) -> Output {
        let exec = self.get_executable();
        dbg!(&exec);
        Command::new(exec)
            .arg(args)
            .output()
            .expect("Failed to execute command!")
    }

    pub fn render(&mut self, project: &ProjectFile, frame: i32) -> Result<PathBuf> {
        let path = project.file_path().to_str().unwrap();

        let mut tmp = ServerSetting::default().blender_data.path.clone();
        tmp.push(format!("{}_{}.png", project.file_name, frame));

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
        let cmd = format!(" -b {} -o {} -f {}", path, output, frame);
        // we'll figure out what to do with this output...
        let _output = Self::exec_command(self, &cmd);

        // display the output and see what result we'll get
        dbg!(_output);

        Ok(tmp)
    }
}

impl Blender<NotInstalled> {
    #[allow(dead_code)]
    fn download(&mut self) -> Result<()> {
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
}
