use crate::{args::Args, mode::Mode};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{io::Result, path::PathBuf, process::Command};

#[derive(Debug, Eq, Serialize, Deserialize)]
pub struct Blender {
    #[allow(dead_code)]
    version: Version, // Use this to help indicate what version blender is installed.
    executable: PathBuf,
}

impl Blender {
    #[allow(dead_code)]
    pub fn new(executable: PathBuf, version: Version) -> Self {
        Blender {
            executable,
            version,
        }
    }

    pub fn from_executable(executable: PathBuf) -> Result<Self> {
        // this should return the version number
        // macos
        let exec = executable.to_str().unwrap();
        let output = Command::new(exec).arg("-v").output()?;
        // Is there a way to handle stdout?
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

    // Render one frame - can we make the assumption that ProjectFile may have configuration predefined Or is that just a system global setting to apply on?
    pub fn render(&self, args: &Args) -> Result<PathBuf> {
        // More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#argument-order
        let path = args.file.to_str().unwrap();
        let output = args.output.to_str().unwrap();
        let mut col = vec![
            "-b".to_owned(),
            path.to_string(),
            "-o".to_owned(),
            output.to_string(),
        ];

        /*
        -F :format::Format
        -x // use extension
        # is substitute to 0 pad, none will add to suffix four pounds (####)
        */

        // this argument must be set at the very end
        let mut additional_args = match args.mode {
            Mode::Frame(f) => {
                vec!["-f".to_owned(), f.to_string()]
            }
            // Render the whole animation using all the settings saved in the blend-file.
            Mode::Animation => {
                vec!["-a".to_owned()]
            }
            Mode::Section(start, end) => vec![
                "-s".to_owned(),
                start.to_string(),
                "-e".to_owned(),
                end.to_string(),
            ],
        };

        col.append(&mut additional_args);

        let output = Command::new(&self.executable)
            .args(col)
            .output()
            .expect("Failed to execute command!");

        let stdout = String::from_utf8(output.stdout).unwrap();
        let col = stdout.split('\n').collect::<Vec<&str>>();
        let location = &col
            .iter()
            .filter(|&x| x.contains("Saved"))
            .collect::<Vec<_>>();
        dbg!(&col);
        let location = location.first().unwrap().split('\'').collect::<Vec<&str>>();
        Ok(PathBuf::from(location[1]))
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
}

impl PartialEq for Blender {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
