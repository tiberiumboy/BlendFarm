/*

Developer blog:
- Re-reading through this code several times, it seems like I got the bare surface working to get started with the rest of the components.
- Thought about if other computer node have identical os/arch - then just distribute the blender downloaded on the source to those target machine instead to prevent multiple downloads from the source.
- I eventually went back and read some parts of Rust Programming Language book to get a better understanding how to handle errors effectively.
- Using thiserror to define custom error within this library and anyhow for main.rs function, eventually I will have to handle those situation of the error message.

- Invoking blender should be called asyncronously on OS thread level. You have the ability to set priority for blender.
*/

use crate::page_cache::PageCache;
use crate::{args::Args, page_cache::PageCacheError};
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env::consts,
    fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};
use thiserror::Error;
use url::Url;

const WINDOW_EXT: &str = ".zip";
const LINUX_EXT: &str = ".tar.xz";
const MACOS_EXT: &str = ".dmg";

// TODO: consider making this private to make it easy to modify internally than affecting exposed APIs
#[derive(Debug, Error)]
pub enum BlenderError {
    #[error("Unable to fetch download from the source! {0}")]
    FetchError(String),
    #[error("Unsupported OS: {0}")]
    UnsupportedOS(String),
    #[error("Unsupported Architecture: {0}")]
    UnsupportedArch(String),
    #[error("Cannot find target download link for blender! os: {os} | arch: {arch} | url: {url}")]
    DownloadNotFound {
        arch: String,
        os: String,
        url: String,
    },
    #[error("Path to executable not found! {0}")]
    ExecutableNotFound(PathBuf),
    #[error("Unable to call blender!")]
    ExecutableInvalid,
    #[error(transparent)]
    PageCache(#[from] PageCacheError),
    #[error("IO Error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlenderHandler {}

/// Blender structure to hold path to executable and version of blender installed.
#[derive(Debug, Eq, Serialize, Deserialize, Clone)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf, // Private immutable variable - Must validate before using!
    /// Version of blender installed on the system.
    pub version: Version, // Private immutable variable - Must validate before using!
                          // possibly a handler to proceed data?
                          // handler: Option<JoinHandle<BlenderHandler>>, // thoughts about passing struct to JoinHandle instead of unit?
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderDownloadLink {
    name: String,
    ext: String,
    url: Url,
}

impl BlenderDownloadLink {
    pub fn new(name: String, ext: String, url: Url) -> Self {
        Self { name, ext, url }
    }

    // Currently being used for MacOS (I wonder if I need to do the same for windows?)
    #[cfg(target_os = "macos")]
    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), BlenderError> {
        fs::create_dir_all(&dst).unwrap();
        for entry in fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                Self::copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name())).unwrap();
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    /// Extract tar.xz file from destination path, and return blender executable path
    #[cfg(target_os = "linux")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, BlenderError> {
        use std::fs::File;
        use tar::Archive;
        use xz::read::XzDecoder;

        // Get file handler to download location
        let file = File::open(&download_path).unwrap();

        // decode compressed xz file
        let tar = XzDecoder::new(file);

        // unarchive content from decompressed file
        let mut archive = Archive::new(tar);

        // generate destination path
        let destination = download_path.as_ref().parent().unwrap();

        // extract content to destination
        archive.unpack(destination).unwrap();

        // return extracted executable path
        Ok(destination.join(folder_name).join("blender"))
    }

    // TODO: Test this on macos
    /// Mounts dmg target to volume, then extract the contents to a new folder using the folder_name,
    /// lastly, provide a path to the blender executable inside the content.
    #[cfg(target_os = "macos")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, BlenderError> {
        use dmg::Attach;

        let source = download_path.as_ref();

        // generate destination path
        let dst = source
            .parent()
            .unwrap()
            .join(folder_name)
            .join("Blender.app");

        // TODO: wonder if this is a good idea?
        if !dst.exists() {
            let _ = fs::create_dir_all(&dst)?;
        }

        // attach dmg to volume
        let dmg = Attach::new(&source).attach()?;

        // create source path from mount point
        let src = PathBuf::from(&dmg.mount_point.join("Blender.app"));

        // Extract content inside Blender.app to destination
        let _ = Self::copy_dir_all(&src, &dst).unwrap();

        // detach dmg volume
        dmg.detach()?;

        // return path with additional path to invoke blender directly
        Ok(dst.join("Contents/MacOS/Blender"))
    }

    // TODO: implement handler to unpack .zip files
    // TODO: Check and see if we need to return the .exe extension or not?
    #[cfg(target_os = "windows")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, BlenderError> {
        let output = download_path.parent().unwrap().join(folder_name);
        todo!("Need to impl. window version of file extraction here");
        Ok(output.join("/blender.exe"))
    }

    pub fn download_and_extract(
        &self,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, BlenderError> {
        let dir = destination.as_ref();

        // Download the file from the internet and save it to blender data folder
        let body = match reqwest::blocking::get(self.url.as_str()) {
            Ok(response) => match response.bytes() {
                Ok(body) => body,
                Err(e) => {
                    return Err(BlenderError::FetchError(format!(
                        "Error while fetching downloads: {}",
                        e
                    )))
                }
            },
            Err(_) => {
                return Err(BlenderError::DownloadNotFound {
                    arch: consts::ARCH.to_string(),
                    os: consts::OS.to_string(),
                    url: self.url.to_string(),
                })
            }
        };

        let target = &dir.join(&self.name);

        if let Err(e) = fs::write(target, &body) {
            return Err(e.into());
        }
        let extract_folder = self.name.replace(&self.ext, "");

        let executable_path = Self::extract_content(target, &extract_folder).unwrap();
        Ok(executable_path)
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
            // handler: None,
        }
    }

    /// Return the pattern matching to identify correct blender download link
    fn generate_blender_pattern_matching(version: &Version) -> Result<String, BlenderError> {
        let extension = Self::get_extension()?;
        let arch = Self::get_valid_arch()?;

        // Regex rules - Find the url that matches version, computer os and arch, and the extension.
        // - There should only be one entry matching for this. Otherwise return error stating unable to find download path
        let match_pattern = format!(
            r#"(<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.{}.*{}.*{}.*\.[{}].*)<\/a>)"#,
            version.major,
            version.minor,
            version.patch,
            consts::OS,
            arch,
            extension
        );

        Ok(match_pattern)
    }

    /// This function will invoke the -v command ot retrieve blender version information.
    ///
    /// # Errors
    /// * InvalidData - executable path do not exist or is invalid. Please verify that the path provided exist and not compressed.
    ///  This error also serves where the executable is unable to provide the blender version.
    // TODO: Find a better way to fetch version from stdout (Possibly regex? How would other do it?)
    // Wonder if this is the better approach? Do not know! We'll find out more?
    fn check_version(executable_path: impl AsRef<Path>) -> Result<Version, BlenderError> {
        let output = match Command::new(executable_path.as_ref()).arg("-v").output() {
            Ok(output) => output,
            Err(_) => return Err(BlenderError::ExecutableInvalid),
        };

        // wonder if there's a better way to test this?
        let regex =
            Regex::new(r"(Blender (?<major>[0-9]).(?<minor>[0-9]).(?<patch>[0-9]))").unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        match regex.captures(&stdout) {
            Some(info) => Ok(Version::new(
                info["major"].parse().unwrap(),
                info["minor"].parse().unwrap(),
                info["patch"].parse().unwrap(),
            )),
            None => Err(BlenderError::ExecutableInvalid),
        }
    }

    /// Return extension matching to the current operating system (Only display Windows(zip), Linux(tar.xz), or macos(.dmg)).
    pub fn get_extension() -> Result<String, BlenderError> {
        // TODO: Find a better way to re-write this - I assume we could take advantage of the compiler tags to return string literal without switch statement like this?
        let extension = match consts::OS {
            "windows" => WINDOW_EXT,
            "macos" => MACOS_EXT,
            "linux" => LINUX_EXT,
            os => return Err(BlenderError::UnsupportedOS(os.to_string())),
        };

        Ok(extension.to_owned())
    }

    /// fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
    pub fn get_valid_arch() -> Result<String, BlenderError> {
        let arch = match consts::ARCH {
            "x86_64" => "64",
            "aarch64" => "arm64",
            value => return Err(BlenderError::UnsupportedArch(value.to_string())),
        };

        Ok(arch.to_owned())
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
        let path = executable.as_ref();
        if !path.exists() {
            return Err(BlenderError::ExecutableNotFound(path.to_path_buf()));
        }

        // currently need a path to the executable before executing the command.
        match Self::check_version(path) {
            Ok(version) => Ok(Self::new(path.to_path_buf(), version)),
            Err(e) => Err(e),
        }
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
        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        let url = Url::parse("https://download.blender.org/release/").unwrap(); // I would hope that this line should never fail...? I would like to know how someone could possibly fail this line here.

        // In the original code - there's a comment implying we should use cache as much as possible to avoid IP Blacklisted. TODO: Verify this in Blender community about this.
        let mut cache = match PageCache::load(SystemTime::now()) {
            Ok(cache) => cache,
            Err(e) => return Err(BlenderError::PageCache(e)),
        };

        // TODO: How did BlendFarm fetch all blender version?
        // working out a hack to rely on website availability for now. Would like to simply get the url I need to download and run blender.
        // could this be made into a separate function?
        let path = format!("Blender{}.{}/", version.major, version.minor);
        let url = url.join(&path).unwrap();

        // fetch the content of the subtree information
        let content = cache.fetch(&url).unwrap();
        let match_pattern = Self::generate_blender_pattern_matching(&version)?;

        // unwrap() is used here explicitly because I know that the above regex command will not fail.
        let regex = Regex::new(&match_pattern).unwrap();
        let download_link = match regex.captures(&content) {
            Some(info) => {
                // remove extension from file name
                let name = info["name"].to_string();
                let path = info["url"].to_string();
                let url = &url.join(&path).unwrap();
                let ext = Self::get_extension().unwrap();
                BlenderDownloadLink::new(name, ext, url.clone())
            }
            None => {
                return Err(BlenderError::DownloadNotFound {
                    arch: consts::ARCH.to_string(),
                    os: consts::OS.to_string(),
                    url: url.to_string(),
                })
            }
        };

        let destination = install_path.as_ref().join(&path);
        fs::create_dir_all(&destination).unwrap();

        // TODO: verify this is working for windows (.zip)?
        let executable = download_link.download_and_extract(&destination)?;

        // return the version of the blender
        Ok(Blender::new(executable, version))
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
    pub fn render(&self, args: &Args) -> Result<String, BlenderError> {
        let col = args.create_arg_list();

        // seems conflicting, this api locks main thread. NOT GOOD!
        // Instead I need to find a way to send signal back to the class that called this
        // and invoke other behaviour once this render has been completed
        // in this case, I shouldn't have to return anything other than mutate itself that it's in progress.
        // modify this struct to include handler for process
        let stdout = Command::new(&self.executable)
            .args(col)
            .stdout(Stdio::piped())
            .spawn()?
            .stdout
            .unwrap();

        let reader = BufReader::new(stdout);
        let mut output: String = Default::default();

        // parse stdout for human to read
        reader.lines().for_each(|line| {
            // it would be nice to include verbose logs?
            let line = line.unwrap();

            if line.contains("Warning:") {
                println!("{}", line);
            } else if line.contains("Fra:") {
                let col = line.split('|').collect::<Vec<&str>>();
                let last = col.last().unwrap().trim();
                let slice = last.split(' ').collect::<Vec<&str>>();
                match slice[0] {
                    "Rendering" => {
                        let current = slice[1].parse::<f32>().unwrap();
                        let total = slice[3].parse::<f32>().unwrap();
                        let percentage = current / total * 100.0;
                        println!("{} {:.2}%", last, percentage);
                    }
                    _ => {
                        println!("{}", last);
                    }
                }
                // this is where I can send signal back to the caller
                // that the render is in progress
            } else if line.contains("Saved:") {
                // this is where I can send signal back to the caller
                // that the render is completed
                // TODO: why this didn't work after second render?
                let location = line.split('\'').collect::<Vec<&str>>();
                dbg!(&line, &location);
                output = location[1].trim().to_string();
            } else {
                // TODO: find a way to show error code or other message if blender doesn't actually render!
                println!("{}", &line);
            }
        });

        Ok(output)
    }
}

impl PartialEq for Blender {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }
}
