use crate::args::Args;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Result},
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Debug, Eq, Serialize, Deserialize)]
pub struct Blender {
    version: Version, // Use this to help indicate what version blender is installed.
    executable: PathBuf,
}

impl Blender {
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
        let output = Command::new(exec).arg("-v").output().unwrap().stdout;
        // Is there a way to handle stdout?
        let stdout = String::from_utf8(output).unwrap();
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
            let line = line.unwrap();
            // println!("{}", &line);
            // if line.contains("Warning:") {
            //     println!("{}", line);
            // } else
            if line.contains("Fra:") {
                let col = line.split('|').collect::<Vec<&str>>();
                let last = col.last().unwrap().trim();
                let slice = last.split(' ').collect::<Vec<&str>>();
                match slice[0] {
                    "Rendering" => {
                        let current = slice[1].parse::<f32>().unwrap();
                        let total = slice[3].parse::<f32>().unwrap();
                        let percentage = current / total * 100.0;
                        println!("Rendering {:.2}%", percentage);
                    }
                    _ => {
                        println!("{}", last);
                    }
                }
                // this is where I can send signal back to the caller
                // that the render is in progress
                // check for either Syncing or Rendering.
            } else if line.contains("Saved:") {
                // this is where I can send signal back to the caller
                // that the render is completed
                let location = line.split('\'').collect::<Vec<&str>>();
                output = location[1].trim().to_string();
                println!("Done! {}", output);
            }
        });

        // self.status
        Ok(output)
    }
}

impl PartialEq for Blender {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }
}
