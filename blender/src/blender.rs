use crate::args::Args;
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
use url::Url;

// TODO - how do I define a constant string argument for url path?
const BLENDER_DOWNLOAD_URL: &str = "https://download.blender.org/release/";
const WINDOW_EXT: &str = ".zip";
const LINUX_EXT: &str = ".tar.xz";
const MACOS_EXT: &str = ".dmg";

/// Blender structure to hold path to executable and version of blender installed.
#[derive(Debug, Eq, Serialize, Deserialize)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf, // Private immutable variable - Must validate before using!
    /// Version of blender installed on the system.
    version: Version, // Private immutable variable - Must validate before using!
}

// TODO: find a way to only allow invocation calls per operating system level
/// Extract tar.xz file from destination path, and return blender executable path
#[cfg(target_os = "linux")]
fn extract_content(download_path: &PathBuf, folder_name: &PathBuf) -> Result<PathBuf> {
    use tar::Archive; // for linux only
    use xz::read::XzDecoder; // possibly used for linux only? Do not know - could verify and check on windows/macos

    // This method only works for tar.xz files (Linux distro)
    // extract the contents of the downloaded file
    let output = download_path.parent().unwrap().join(folder_name);
    let file = File::open(&download_path).unwrap(); // comment this out if we can get the line above working again - wouldn't make sense to open after we created?
    let tar = XzDecoder::new(file);
    let mut archive = Archive::new(tar);
    archive.unpack(&output).unwrap();
    Ok(output.join("/blender"))
}

/// Extract dmg files into destination path, and return the blender executable path
// TODO: verify a way to test this out
// I need to extract the content of dmg before running blender. I fear that I will run into permission issue, so I need to see how BlendFarm C# did it.
// Problem is, I'm on the airplane and wifi expensive as hell, and would need to clone the repo when I get to the destination.
// so I could review the code better and understand exactly how they did it from C# side of the world.
// Also - great opportunity to look into windows section and see if there's anything unusual or unique I need to perform on rust side.
#[cfg(target_os = "macos")]
fn extract_content(download_path: &PathBuf, folder_name: &str) -> Result<PathBuf> {
    use dmgwiz::{DmgWiz, Verbosity};

    let output = download_path.parent().unwrap().join(folder_name);
    let file = File::open(&download_path).unwrap();
    let mut dmg = DmgWiz::from_reader(file, Verbosity::None).unwrap();
    let outfile = File::create(&output.join("test.bin")).unwrap();
    let output_buf = BufWriter::new(outfile);
    // I wonder if I need to provide a destination?
    // return usize (file size?)
    let result = dmg.extract_all(output_buf).unwrap();
    dbg!(result);
    Ok(output.join("/blender"))
}

// TODO: implement handler to unpack .zip files
// TODO: Check and see if we need to return the .exe extension or not?
#[cfg(target_ps = "windows")]
fn extract_content(download_path: &PathBuf, folder_name: &str) -> Result<PathBuf> {
    let output = download_path.parent().unwrap().join(folder_name);
    todo!("Need to impl. window version of file extraction here");
    Ok(output.join("/blender.exe"))
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
        let url = Url::parse(BLENDER_DOWNLOAD_URL).unwrap(); // I would hope that this line should never fail...?
        let path = format!("Blender{}.{}/", version.major, version.minor);
        let url = url.join(&path).unwrap();

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

        // fetch content list from subtree
        let content = match reqwest::blocking::get(url.clone()) {
            Ok(data) => data.text().unwrap(),
            Err(e) => return Err(Error::new(ErrorKind::BrokenPipe, e.to_string())),
        };

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

        // concatenate the final download destination to the url path
        let url = url.join(&path).unwrap();

        // remove extension from file name
        let name = name.replace(extension, "");
        let download_path = install_path.join(&path);

        // it would be nice to ask reqwest to save the content instead of having to transfer from memory over...
        // TODO: something wrong with this codeblock - it download "something", but unable to extract the content in it?
        // let response = reqwest::blocking::get(url).unwrap();
        // let body = response.text().unwrap();
        // let mut file = File::create(&download_path).unwrap();
        // io::copy(&mut body.as_bytes(), &mut file).expect("Unable to write file! Permission issue?");

        // TODO: Need to verify that I can extract dmg files on macos.
        // TODO: verify this is working for macos (.dmg) and windows (.zip)
        let executable = extract_content(&download_path, &name).unwrap();
        // let executable = PathBuf::from("./");

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
