use crate::args::Args;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{BufRead, BufReader, Result},
    path::PathBuf,
    process::{Command, Stdio},
};
use url::Url;

// TODO - how do I define a constant string argument for url path?
const BLENDER_DOWNLOAD_URL: &str = "https://download.blender.org/release/";

/// Blender structure to hold path to executable and version of blender installed.
#[derive(Debug, Eq, Serialize, Deserialize)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf, // Private immutable variable - Must validate before using!
    /// Version of blender installed on the system.
    version: Version, // Private immutable variable - Must validate before using!
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
        Blender {
            executable,
            version,
        }
    }

    /// Create a new blender struct from executable path. This function will fetch the version of blender by invoking -v command.
    /// Otherwise, if Blender is not install, or a version is not found, an error will be thrown
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::from_executable(Pathbuf::from("path/to/blender")).unwrap();
    /// ```
    pub fn from_executable(executable: PathBuf) -> Result<Self> {
        let exec = executable.as_path();
        let output = Command::new(exec).arg("-v").output().unwrap().stdout;
        let stdout = String::from_utf8(output).unwrap();
        let collection = stdout.split("\n\t").collect::<Vec<&str>>();
        let first = collection.first().unwrap();
        let version = if first.contains("Blender") {
            Version::parse(&first[8..]).unwrap() // this looks sketchy...
        } else {
            Version::new(4, 1, 0) // still sketchy, but it'll do for now
        };

        Ok(Blender::new(executable, version))
    }

    /// Download blender from the internet and install it to the provided path.
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::download(Version::new(4,1,0), PathBuf::from("path/to/installation")).unwrap();
    /// ```
    pub fn download(version: Version, install_path: PathBuf) -> Result<Blender> {
        // then use the install_path to install blender directly.
        // TODO: Find a way to utilize extracting utility to unzip blender after download has complete.
        let url = Url::parse(BLENDER_DOWNLOAD_URL).unwrap();
        let path = format!("Blender{}.{}/", version.major, version.minor);
        let url = url.join(&path).unwrap();

        // this OS includes the operating system name and the compressed format.
        let os = match env::consts::OS {
            "windows" => Ok(("windows".to_string(), "zip".to_string())),
            "macos" => Ok(("macos".to_string(), "dmg".to_string())),
            "linux" => Ok(("linux".to_string(), "tar.xy".to_string())),
            // Currently unsupported OS because blender does not have the toolchain to support OS.
            // It may be available in the future, but for now it's currently unsupported as of today.
            // TODO: See if some of the OS can compile and run blender natively, android/ios/freebsd?
            // - ios - Apple OS - may not support - https://en.wikipedia.org/wiki/IOS - requires MacOS / xcode to compile.
            // - freebsd - see below - https://www.freebsd.org/
            // - dragonfly - may be supported? may have to compile open source blender - https://www.dragonflybsd.org/
            // - netbsd - may be supported? See toolchain links and compiling blender from open source - https://www.netbsd.org/
            // - openbsd - may be supported? See toolchain links and compiling blender from Open source - https://www.openbsd.org/
            // - solaris - Oracle OS - may not support - https://en.wikipedia.org/wiki/Oracle_Solaris
            // - android - may be supported? See ARM instruction.
            _ => Err(format!("Unsupported OS! {}", env::consts::OS)),
        };

        // fetch current architecture (Currently support 64bit or arm64)
        let arch = match env::consts::ARCH {
            // "x86" => Ok("32"),
            "x86_64" => Ok("64"),
            "aarch64" => Ok("arm64"),
            // - arm - Not sure where this one will be used or applicable? TODO: Future research - See if blender does support ARM processor and if not, fall under unsupported arch?
            // - powerpc  - TODO: research if this is widely used? may support? Do not know yet. - https://en.wikipedia.org/wiki/PowerPC
            // - powerpc64  - TODO: research if this is widely used? Similar to above, support 64 bit architecture
            // - riscv64  - TODO: research if this is widely used? https://en.wikipedia.org/wiki/RISC-V
            // - s390x - TODO: research if this is widely used?
            // - sparc64  - TODO: research if this is widely used?
            _ => Err(format!(
                "Unsupported architecture found! {}",
                env::consts::ARCH
            )),
        };

        // fetch content list from subtree
        let content = reqwest::blocking::get(url)
            .expect("unable to fetch content from the internet! Is the firewall blocking it or are you connected?")
            .text()
            .unwrap();

        // Content parsing to get download url that matches target operating system and version
        let os = os.unwrap();
        let match_pattern = format!(
            r#"(<a href="(?<url>.*?)".*{}.*{}.*{}.*.{}*</a>.)"#,
            version,
            os.0,
            arch.unwrap(),
            os.1
        );
        let regex = Regex::new(&match_pattern).unwrap();
        let url = match regex.captures(&content) {
            Some(info) => info["url"].to_string(),
            None => panic!("Unable to find the download link!"),
        };
        dbg!(url);

        // now run reqwest on the url, and fetch the current links to find the url that matches the pattern above.

        let executable = install_path.join("blender");

        // download blender from the internet
        // extract the blender to a temporary directory
        // return the path to the blender executable
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
    /// ```
    pub fn render(&mut self, args: &Args) -> Result<String> {
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
            // println!("{}", &line);
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
                let location = line.split('\'').collect::<Vec<&str>>();
                output = location[1].trim().to_string();
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
