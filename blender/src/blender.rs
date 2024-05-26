use crate::args::Args;
use dmgwiz::{DmgWiz, Verbosity}; // for macOS only
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Result},
    path::PathBuf,
    process::{Command, Stdio},
};
// use tar::Archive; // for linux only
// use xz::read::XzDecoder; // possibly used for linux only? Do not know - could verify and check on windows/macos
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
            "windows" => Ok(("windows".to_string(), ".zip".to_string())),
            "macos" => Ok(("macos".to_string(), ".dmg".to_string())),
            "linux" => Ok(("linux".to_string(), ".tar.xz".to_string())),
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
        // Linux and macos works as intended
        let arch = match env::consts::ARCH {
            // "x86" => Ok("32"),
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
                    format!("Unsupported architecture found! {}", env::consts::ARCH),
                ))
            }
        };

        // fetch content list from subtree
        // let content = reqwest::blocking::get(url.clone())
        //     .expect("unable to fetch content from the internet! Is the firewall blocking it or are you connected?")
        //     .text()
        //     .unwrap();
        // TODO: this line works for linux - but does not work for macos. figure out why?
        let content_path = match env::consts::OS {
            "linux" | "macos" => PathBuf::from("./src/examples/Blender3.0.html"),
            _ => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    format!("Not yet supported for {}", env::consts::OS),
                ))
            }
        };

        let content = fs::read_to_string(&content_path).unwrap();

        // Content parsing to get download url that matches target operating system and version
        let os = os.unwrap();
        let match_pattern = format!(
            r#"(<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.{}.*{}.*{}.*\.[{}].*)<\/a>)"#,
            version.major, version.minor, version.patch, os.0, arch, os.1
        );

        let regex = Regex::new(&match_pattern).unwrap();
        // TODO: also fetch the "name" from regex - in case the name doesn't match the same as href
        let path = match regex.captures(&content) {
            Some(info) => info["url"].to_string(),
            // TODO: find a way to gracefully error out of this function call.
            None => panic!("Unable to find the download link!"),
        };

        // concatenate the final download destination to the url path
        let url = url.join(&path).unwrap();

        // create download path location
        let download_path = install_path.join(&path);

        dbg!(&download_path);

        // it would be nice to ask reqwest to save the content instead of having to transfer from memory over...
        // TODO: something wrong with this codeblock - it download "something", but unable to extract the content in it?
        // let response = reqwest::blocking::get(url).unwrap();
        // let body = response.text().unwrap();
        // let mut file = File::create(&download_path).unwrap();
        // io::copy(&mut body.as_bytes(), &mut file).expect("Unable to write file! Permission issue?");

        // This method only works for tar.xz files (Linux distro)
        // extract the contents of the downloaded file
        // let file = File::open(&download_path).unwrap(); // comment this out if we can get the line above working again - wouldn't make sense to open after we created?
        // let tar = XzDecoder::new(file);
        // let mut archive = Archive::new(tar);
        // archive.unpack(&install_path).unwrap();

        // This method only works for .dmg files (macos)
        let file = File::open(&download_path).unwrap();
        let mut dmg = DmgWiz::from_reader(file, Verbosity::None).unwrap();
        let outfile = File::create(&install_path.join("test.bin")).unwrap();
        let output = BufWriter::new(outfile);
        // I wonder if I need to provide a destination?
        // return usize (file size?)
        let result = dmg.extract_all(output).unwrap();
        dbg!(result);

        // Linux and macos works as intended -
        // TODO: Need to verify that I can extract dmg files on macos.

        let dir = path.replace(&os.1, "");
        let executable = install_path.join(dir).join("blender");

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
