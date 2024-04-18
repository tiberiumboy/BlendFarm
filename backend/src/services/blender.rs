use crate::models::{project_file::ProjectFile, server_setting::ServerSetting};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{io::Result, marker::PhantomData, path::PathBuf, process::Command};
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
//

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
            Version::parse(&first[8..]).unwrap() // this looks sketchy...
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

    // going to ignore this for now and figure out what I need to get this working again.
    /*
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
}

impl Blender<Installed> {
    fn get_executable(&self) -> &str {
        // we'll find a way to get the executable if all else fails
        self.executable.as_ref().unwrap().to_str().unwrap()
    }

    pub fn render(&mut self, project: &ProjectFile, frame: i32) -> Result<PathBuf> {
        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        let path = project.file_path().to_str().unwrap();

        let tmp = ServerSetting::default().render_data.path.clone();
        let output = format!("{}/", tmp.as_os_str().to_str().unwrap()); // needs a directory ending - otherwise it will use directory name as file name instead.

        /*
        --log "*" to log everything
        -F :format::Format
        -E ::Engine,
        -x // use extension
        # is substitute to 0 pad, none will add to suffix four pounds (####)
        */
        // let output = Self::exec_command(self, &cmd);
        let output = format!("-o {}", &output); // output path
        let frame = format!("-f {}", &frame.to_string()); // Frame number
        let exec = self.get_executable();
        dbg!(&exec);
        let output = Command::new(exec)
            .arg("-b")
            .arg(path)
            .arg(&output)
            .arg(&frame)
            .output()
            .expect("Failed to execute command!");

        // display the output and see what result we'll get
        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        dbg!(&stdout);
        let col = stdout.split('\n').collect::<Vec<&str>>();
        let location = &col
            .iter()
            .filter(|&x| x.contains("Saved"))
            .collect::<Vec<_>>();

        let location = location.first().unwrap().split('\'').collect::<Vec<&str>>();

        dbg!(location);

        Ok(PathBuf::from("test"))
        // Ok(PathBuf::from(location[1]))
    }
}

impl Blender<NotInstalled> {
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
}
