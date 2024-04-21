use crate::models::{project_file::ProjectFile, server_setting::ServerSetting};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{io::Result, marker::PhantomData, path::PathBuf, process::Command};
use url::Url;

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
//

// const CACHE_DAYS: u8 = 3;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blender {
    version: Version, // version of blender
    executable: PathBuf,
}

// url to download repository
const VERSIONS_URL: &str = "https://download.blender.org/release/";

impl Blender {
    pub fn from_executable(executable: PathBuf) -> Result<Self> {
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
            Version::parse(&first[8..]).unwrap() // this looks sketchy...
        } else {
            Version::new(4, 1, 0)
        };
        // still sketchy, but it'll do for now

        Ok(Blender {
            executable, // is this necessary?
            version,
        })
    }
    // going to ignore this for now and figure out what I need to get this working again.
    /*


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

    */

    // Render one frame - can we make the assumption that ProjectFile may have configuration predefined Or is that just a system global setting to apply on?
    pub fn render(&mut self, project: &ProjectFile, frame: i32) -> Result<PathBuf> {
        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        let path = project.file_path().to_str().unwrap();
        let frame = frame.to_string();

        // creates a temp directory
        let tmp = ServerSetting::default().render_data.path;
        let output = tmp.as_path().as_os_str().to_str().unwrap(); // needs a directory ending - otherwise it will use directory name as file name instead.
        let args = vec!["-b", path, "-o", output, "-f", frame.as_str()]; //", path, output, &frame.to_string()];

        /*
        --log "*" to log everything
        -F :format::Format
        -E ::Engine,
        -x // use extension
        # is substitute to 0 pad, none will add to suffix four pounds (####)
        */

        let output = Command::new(&self.executable)
            .args(args)
            .output()
            .expect("Failed to execute command!");

        let stdout = String::from_utf8(output.stdout).unwrap();
        let col = stdout.split('\n').collect::<Vec<&str>>();
        let location = &col
            .iter()
            .filter(|&x| x.contains("Saved"))
            .collect::<Vec<_>>();

        let location = location.first().unwrap().split('\'').collect::<Vec<&str>>();
        Ok(PathBuf::from(location[1]))
    }
}

// TODO: Create a new struct to perform Blender version checks. Push remaining code from laptop when possible - but remove sensitive information such as server ip address
// impl Blender<NotInstalled> {
// fn get_archive_name(&self) -> String {
//     let env = env::consts::OS;
//     let os = match env {
//         "linux" => "linux64".to_owned(),
//         "windows" => "windows64".to_owned(),
//         "macos" => "macOS".to_owned(),
//         _ => "unknown".to_owned(),
//     };

//     let ext = match env {
//         "linux" => "tar.xz".to_owned(),
//         "windows" => "zip".to_owned(),
//         "macos" => "dmg".to_owned(),
//         _ => "unknown".to_owned(), // would it be nice if blender could run on arm?
//     };

//     // todo - correct arch labeling, e.g. x86_64 -> x64, arm -> arm64, etc
//     let arch = env::consts::ARCH;
//     let archive = format!("blender-{}-{}-{}.{}", self.version, os, arch, ext);

//     dbg!(&archive);

//     archive
// }

// #[allow(dead_code)]
// fn download(&mut self) -> Result<()> {
//     if self.is_cached() {
//         return Ok(());
//     }

//     let config = ServerSetting::default();
//     let archive_name = self.get_archive_name();
//     let archive_path = PathBuf::from(&config.blender_data.path).join(&archive_name);

//     dbg!(&archive_path);

//     // download the file first then set the dl_content afterward once the file has completed the download.
//     self.dl_content = Some(archive_path);

//     Ok(())
// }

// fn install(&mut self) -> Blender<Installed> {
//     // todo - install the downloaded file
//     if !self.is_cached() {
//         self.download();
//     }

//     let archive = self.dl_content.unwrap();
// }
