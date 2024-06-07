use crate::args::Args;
use crate::page_cache::PageCache;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{BufRead, BufReader, Error, ErrorKind, Result},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use url::Url;

// TODO - See aboout an offline regex search engine?
const BLENDER_DOWNLOAD_URL: &str = "https://download.blender.org/release/";
const WINDOW_EXT: &str = ".zip";
const LINUX_EXT: &str = ".tar.xz";
const MACOS_EXT: &str = ".dmg";

/// Blender structure to hold path to executable and version of blender installed.
#[derive(Debug, Eq, Serialize, Deserialize, Clone)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf, // Private immutable variable - Must validate before using!
    /// Version of blender installed on the system.
    pub version: Version, // Private immutable variable - Must validate before using!
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

    /// Returns true when the executable path exist and leads to a blender executable location
    /// This function does not validate whether we can execute and run blender from this executable location.
    ///
    /// # Example
    ///
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::new(PathBuf::from("path/to/blender"), Version::new(4,1,0));
    /// if blender.exists() {
    ///     Ok(())
    /// } else {
    ///     Err("Does not exist!")
    /// }
    /// ```
    pub fn exists(&self) -> bool {
        self.executable.exists()
    }

    // Currently being used for MacOS (I wonder if I need to do the same for windows?)
    #[cfg(target_os = "macos")]
    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
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
    fn extract_content(download_path: &PathBuf, folder_name: &str) -> Result<PathBuf> {
        use std::fs::File;
        use tar::Archive;
        use xz::read::XzDecoder;

        // Get file handler to download location
        let file = File::open(download_path).unwrap();

        // decode compressed xz file
        let tar = XzDecoder::new(file);

        // unarchive content from decompressed file
        let mut archive = Archive::new(tar);

        // generaet destination path
        let destination = download_path.parent().unwrap().join(folder_name);

        // extract content to destination
        archive.unpack(&destination).unwrap();

        // return extracted executable path
        Ok(destination.join("blender"))
    }

    /// Mounts dmg target to volume, then extract the contents to a new folder using the folder_name,
    /// lastly, provide a path to the blender executable inside the content.
    #[cfg(target_os = "macos")]
    fn extract_content(download_path: &PathBuf, folder_name: &str) -> Result<PathBuf> {
        use dmg::Attach;

        // generate destination path
        let dst = download_path
            .parent()
            .unwrap()
            .join(folder_name)
            .join("Blender.app");

        // TODO: wonder if this is a good idea?
        if !dst.exists() {
            let _ = fs::create_dir_all(&dst)?;
        }

        // attach dmg to volume
        let dmg = Attach::new(download_path).attach()?;

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
    #[cfg(target_ps = "windows")]
    fn extract_content(download_path: &PathBuf, folder_name: &str) -> Result<PathBuf> {
        let output = download_path.parent().unwrap().join(folder_name);
        todo!("Need to impl. window version of file extraction here");
        Ok(output.join("/blender.exe"))
    }

    /// This function will invoke the -v command ot retrieve blender version information.
    ///
    /// # Errors
    /// * InvalidData - executable path do not exist or is invalid. Please verify that the path provided exist and not compressed.
    ///  This error also serves where the executable is unable to provide the blender version.
    // TODO: Find a better way to fetch version from stdout (Possibly regex? How would other do it?)
    // Wonder if this is the better approach? Do not know! We'll find out more?
    fn check_version(executable_path: impl AsRef<Path>) -> Result<Version> {
        let output = Command::new(executable_path.as_ref())
            .arg("-v")
            .output()
            .unwrap();

        // wonder if there's a better way to test this?
        let regex =
            Regex::new(r"(Blender (?<major>[0-9]).(?<minor>[0-9]).(?<patch>[0-9]))").unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        match regex.captures(&stdout) {
            Some(info) => Ok(Version::new(info["major"].parse().unwrap(), info["minor"].parse().unwrap(), info["patch"].parse().unwrap())),
            None => Err(Error::new(
                ErrorKind::NotFound,
                    "Unable to fetch blender version! Are you sure you provided the exact blender executable path?",
            )),
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
    pub fn from_executable(executable: impl AsRef<Path>) -> Result<Self> {
        // check and verify that the executable exist.
        let path = executable.as_ref();
        if !path.exists() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Executable path do not exist or is invalid!",
            ));
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
    pub fn download(version: Version, install_path: impl AsRef<Path>) -> Result<Blender> {
        let url = Url::parse(BLENDER_DOWNLOAD_URL).unwrap(); // I would hope that this line should never fail...? I would like to know how someone could possibly fail this line here.

        // In the original code - there's a comment implying we should use cache as much as possible to avoid IP Blacklisted. TODO: Verify this in Blender community about this.
        let mut cache = PageCache::load();

        // create a subpath using the version and check to see if this exist. Otherwise, I may have to regex this information out...?
        // TODO: Once I get internet connection, finish this - Impl cache for the download page, impl regex search for specific blender version
        let content: String = cache.fetch(&url).unwrap();

        // search for the root of the blender version
        // does it seems important? How did BlendFarm fetch all blender version?
        let path = format!("Blender{}.{}/", version.major, version.minor);
        let url = url.join(&path).unwrap();

        let blender = Blender::new(PathBuf::from("path/to/blender"), version);
        Ok(blender)

        /*

        // fetch the content of the subtree information
        let content = cache.fetch(&url).unwrap();

        dbg!(&content);

        // this OS includes the operating system name and the compressed format.
        // TODO: might reformat this so that it make sense - I don't think I need to capture the OS string literal, but I do need to capture the file extension type.
        let (os, extension) = match env::consts::OS {
            "windows" => ("windows".to_string(), WINDOW_EXT),
            "macos" => ("macos".to_string(), MACOS_EXT),
            "linux" => ("linux".to_string(), LINUX_EXT),
            // Currently unsupported OS because blender does not have the toolchain to support OS.
            // It may be available in the future, but for now it's currently unsupported as of today.
            // TODO: See if some of the OS can compile and run blender natively, android/ios/freebsd?
            // - ios - Apple OS - may not support - https://en.wikipedia.org/wiki/IOS - requires MacOS / xcode to compile.
            // - freebsd - see below - https://www.freebsd.org/
            // - dragonfly - may be supported? may have to compile open source blender - https://www.dragonflybsd.org/
            // - netbsd - may be supported? See toolchain links and compiling blender from open source - https://www.netbsd.org/
            // - openbsd - may be supported? See toolchain links and compiling blender from Open source - https://www.openbsd.org/
            // - solaris - Oracle OS - may not support - https://en.wikipedia.org/wiki/Oracle_Solaris
            // - android - may be supported? See ARM instruction. - Do not know if we need to run specific toollink to compile for ARM processors.
            _ => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    format!("No support for {}!", env::consts::OS),
                ))
            }
        };

        // fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
        let arch = match env::consts::ARCH {
            // "x86" => "32", // newer version of blender no longer support 32 bit arch - but older version does. Let's keep this in tact just in case.
            "x86_64" => "64",
            "aarch64" => "arm64",
            // - arm - Not sure where this one will be used or applicable? TODO: Future research - See if blender does support ARM processor and if not, fall under unsupported arch?
            // - powerpc  - TODO: research if this is widely used? may support? Do not know yet. - https://en.wikipedia.org/wiki/PowerPC
            // - powerpc64  - TODO: research if this is widely used? Similar to above, support 64 bit architecture
            // - riscv64  - TODO: research if this is widely used? https://en.wikipedia.org/wiki/RISC-V
            // - s390x - TODO: research if this is widely used?
            // - sparc64  - TODO: research if this is widely used?
            _ => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    format!(
                        r#"No support for target architecture "{}""#,
                        env::consts::ARCH
                    ),
                ))
            }
        };

        dbg!(&os, &extension, &arch);

        // Content parsing to get download url that matches target operating system and version
        let match_pattern = format!(
            r#"(<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.{}.*{}.*{}.*\.[{}].*)<\/a>)"#,
            version.major, version.minor, version.patch, os, arch, extension
        );

        let regex = Regex::new(&match_pattern).unwrap();
        let (path, name) = match regex.captures(&content) {
            Some(info) => (info["url"].to_string(), info["name"].to_string()),
            None => return Err(Error::new(
                ErrorKind::NotFound,
                format!(
                    "Unable to find the download link for target platform! OS: {} | Arch: {} | Version: {} | url: {}",
                    os, arch, version, url
                ),
            )),
        };
        dbg!(&path, &name);

        // concatenate the final download destination to the url path
        let url = url.join(&path).unwrap();

        // remove extension from file name
        let name = name.replace(extension, "");
        let download_path = install_path.as_ref().join(&path);

        dbg!(&download_path, &url);
        // Download the file from the internet and save it to blender data folder
        // I feel like I'm running into problem here?
        let response = match reqwest::blocking::get(url) {
            Ok(response) => response,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        let body = response.bytes().unwrap();
        fs::write(&download_path, &body).expect("Unable to write file! Permission issue?");

        // TODO: verify this is working for windows (.zip)
        let executable = extract_content(&download_path, &name).unwrap();

        // return the version of the blender
        Ok(Blender::new(executable, version))

        */
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
    pub fn render(&self, args: &Args) -> Result<String> {
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
